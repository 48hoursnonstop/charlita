#include "bridge.h"
#include <QApplication>
#include <QDir>
#include <QImage>
#include <QImageReader>
#include <QJsonDocument>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QProcess>
#include <QStandardPaths>
#include <QQmlApplicationEngine>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTcpServer>
#include <QTemporaryDir>
#include <QtTest>
#include <memory>
class UiTests final : public QObject {
    Q_OBJECT
    std::unique_ptr<QTemporaryDir> folder;
    std::unique_ptr<Bridge> backend;
    std::unique_ptr<QQmlApplicationEngine> qml;
    QQuickWindow *window = nullptr;
    QString groupId;
    QVariantMap document() const { return backend->state()["document"].toMap(); }
    static QQuickItem *findVisualItem(QQuickItem *root, const QString &name) {
        if(root->objectName()==name)return root;
        for(auto *child:root->childItems())
            if(auto *found=findVisualItem(child,name))return found;
        return nullptr;
    }
    QQuickItem *item(const char *name) const { return findVisualItem(window->contentItem(),QString::fromUtf8(name)); }
    QJSValue evaluate(const QString &script) { return qml->evaluate(script); }
    void capture(const QString &name, int waitMs=80) {
        const auto directory=qEnvironmentVariable("CHARLITA_UI_CAPTURE_DIR");
        if (directory.isEmpty()) return;
        QDir().mkpath(directory);if(waitMs)QTest::qWait(waitMs);
        QVERIFY(window->grabWindow().save(directory+"/"+name+".png"));
    }
    void click(QQuickItem *item) {
        QVERIFY(item);
        const auto point = item->mapToScene(QPointF(item->width()/2,item->height()/2)).toPoint();
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier, point);
    }
    QVariantMap publishedOutput() {
        QNetworkAccessManager manager;
        auto *reply=manager.get(QNetworkRequest(QUrl(backend->state()["baseUrl"].toString()+"/state/group/"+groupId)));
        QSignalSpy done(reply,&QNetworkReply::finished);
        if (!reply->isFinished() && !done.wait(3000)) return {};
        return QJsonDocument::fromJson(reply->readAll()).object().toVariantMap();
    }
    QVariantMap publishedGuest() {
        const auto guests=publishedOutput()["guests"].toList();return guests.isEmpty()?QVariantMap{}:guests.first().toMap();
    }
private slots:
    void initTestCase() { QApplication::setQuitOnLastWindowClosed(false);QQuickStyle::setStyle("Basic"); }
    void init() {
        folder=std::make_unique<QTemporaryDir>();QVERIFY(folder->isValid());
        QTcpServer port;QVERIFY(port.listen(QHostAddress::LocalHost,0));const auto number=port.serverPort();port.close();
        backend=std::make_unique<Bridge>(QVariantMap{{"data_dir",folder->path()},{"port",number},{"auto_check",false}});
        QVERIFY2(backend->ready(),qPrintable(backend->error()));
        groupId=document()["profiles"].toList().first().toMap()["groups"].toList().first().toMap()["id"].toString();
        qml=std::make_unique<QQmlApplicationEngine>();
        qml->setInitialProperties({{"backend",QVariant::fromValue(backend.get())}});
        qml->loadFromModule("CharlitaTests","Main");QVERIFY(!qml->rootObjects().isEmpty());
        window=qobject_cast<QQuickWindow *>(qml->rootObjects().first());QVERIFY(window);
        window->resize(1220,820);window->show();QTRY_VERIFY(window->isVisible());
        qml->globalObject().setProperty("window",qml->newQObject(window));
        capture("empty");
    }
    void cleanup() { qml.reset();window=nullptr;backend.reset();folder.reset(); }
    void keyboardGuestEditingPublishesOnlyWithApply() {
        QVERIFY(backend->request("add_guest",{{"group",groupId},{"user","123456789012345678"},{"name","Before"}})["ok"].toBool());
        QVERIFY(backend->request("apply")["ok"].toBool());
        QCOMPARE(publishedGuest()["name"].toString(),QString("Before"));
        QVERIFY(!evaluate("window.showGuest('123456789012345678')").isError());
        auto *drawer=window->findChild<QObject *>("guestEditor");QVERIFY(drawer);QTRY_VERIFY(drawer->property("position").toReal()>0.99);
        auto *field=item("guestName");QVERIFY(field);click(field);QTRY_VERIFY(field->hasActiveFocus());
        QTest::keySequence(window,QKeySequence::SelectAll);
        for(char key:QByteArray("After"))QTest::keyClick(window,key);
        QTest::keyClick(window,Qt::Key_Return);
        QTRY_COMPARE(document()["people"].toMap()["123456789012345678"].toMap()["name"].toString(),QString("After"));
        capture("guest-editor");
        QCOMPARE(publishedGuest()["name"].toString(),QString("Before"));
        click(item("closeGuest"));QTRY_VERIFY(!drawer->property("visible").toBool());
        auto *apply=item("applyChanges");QSignalSpy applied(apply,SIGNAL(clicked()));
        click(apply);QCOMPARE(applied.count(),1);QTRY_COMPARE(publishedGuest()["name"].toString(),QString("After"));
        QVERIFY(!backend->state()["dirty"].toBool());
        capture("applied");
    }
    void importedArtworkAppearsInLibraryAndOpensEditor() {
        const auto path=folder->filePath("fixture.png");QImage image(32,32,QImage::Format_RGBA8888);image.fill(QColor("#b4db91"));QVERIFY(image.save(path));
        QVERIFY(backend->request("import_art",{{"path",path}})["ok"].toBool());
        QTRY_VERIFY_WITH_TIMEOUT(!backend->state()["busy"].toBool(),5000);
        QCOMPARE(document()["characters"].toList().size(),1);
        const auto id=document()["characters"].toList().first().toMap()["id"].toString();
        window->setProperty("page",1);QTest::qWait(40);
        capture("library");
        QVERIFY(!evaluate(QString("window.showCharacter('%1')").arg(id)).isError());
        auto *drawer=window->findChild<QObject *>("characterEditor");QVERIFY(drawer);QTRY_VERIFY(drawer->property("position").toReal()>0.99);
        QCOMPARE(drawer->property("asset").toMap()["width"].toInt(),32);
        capture("character-editor");
        window->resize(320,640);QTest::qWait(80);capture("character-narrow");
        for (int i=0;i<4;++i) {
            auto *crop=item(qPrintable(QString("cropControl%1").arg(i)));QVERIFY(crop);
            const auto rect=crop->mapRectToScene(QRectF(0,0,crop->width(),crop->height()));
            QVERIFY2(rect.left()>=0&&rect.right()<=window->width(),"Crop control exceeds the narrow window");
        }
        for (int i=0;i<35;++i) {
            QTest::keyClick(window,Qt::Key_Tab);QCoreApplication::processEvents();
            auto *focus=window->activeFocusItem();QVERIFY(focus);
            const auto rect=focus->mapRectToScene(QRectF(0,0,focus->width(),focus->height()));
            QVERIFY2(rect.left()>=0&&rect.right()<=window->width()&&rect.top()>=0&&rect.bottom()<=window->height(),qPrintable(QString("Focused item outside window: %1 x=%2 right=%3 y=%4 bottom=%5").arg(focus->objectName()).arg(rect.left()).arg(rect.right()).arg(rect.top()).arg(rect.bottom())));
        }
        capture("character-keyboard-narrow");
        window->resize(1220,820);
        QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!drawer->property("visible").toBool());
        window->setProperty("page",0);
        QVERIFY(backend->request("add_guest",{{"group",groupId},{"user","123456789012345678"},{"name","Ariadna"}})["ok"].toBool());
        QVERIFY(!evaluate(QString("window.editPerson('123456789012345678', p => p.character = '%1')").arg(id)).isError());
        QVERIFY(publishedGuest().isEmpty());
        auto *apply=item("applyChanges");QTRY_VERIFY(apply->isVisible());
        QSignalSpy applied(apply,SIGNAL(clicked()));click(apply);QCOMPARE(applied.count(),1);
        QTRY_VERIFY(!publishedGuest()["media"].toMap()["url"].toString().isEmpty());
        capture("imported-applied");
        auto *scene=window->findChild<QObject *>("scenePage");QVERIFY(scene);
        scene->setProperty("expanded",true);capture("expanded");
    }
    void smallWindowKeepsPrimaryActionAndKeyboardFocusReachable() {
        QVERIFY(backend->request("add_guest",{{"group",groupId},{"user","123456789012345678"},{"name","Guest"}})["ok"].toBool());
        window->resize(320,640);QTest::qWait(100);
        auto *apply=item("applyChanges");QVERIFY(apply);
        QVERIFY(apply->isEnabled());apply->forceActiveFocus(Qt::TabFocusReason);
        QTRY_VERIFY(apply->hasActiveFocus());capture("apply-focus");
        const auto rect=apply->mapRectToScene(QRectF(0,0,apply->width(),apply->height()));
        QVERIFY2(rect.left()>=0&&rect.right()<=window->width(),qPrintable(QString("Apply control outside window: %1,%2").arg(rect.left()).arg(rect.right())));
        auto *status=item("applyStatus");QVERIFY(status);
        const auto statusRect=status->mapRectToScene(QRectF(0,0,status->width(),status->height()));
        QVERIFY(statusRect.left()>=0&&statusRect.right()<=window->width());
        for(int i=0;i<8;++i)QTest::keyClick(window,Qt::Key_Tab);
        QVERIFY(window->activeFocusItem());
        QVERIFY(window->activeFocusItem()->isVisible());
        capture("narrow");
        window->setProperty("page",2);capture("settings-narrow");
    }
    void importErrorRemainsReachableOverAnEditingDrawer() {
        const auto path=folder->filePath("valid.png");QImage artwork(32,32,QImage::Format_RGBA8888);artwork.fill(Qt::red);QVERIFY(artwork.save(path));
        QVERIFY(backend->request("import_art",{{"path",path}})["ok"].toBool());
        QTRY_VERIFY(!backend->state()["busy"].toBool());
        const auto id=document()["characters"].toList().first().toMap()["id"].toString();
        QVERIFY(!evaluate(QString("window.showCharacter('%1')").arg(id)).isError());
        auto *drawer=window->findChild<QObject *>("characterEditor");QVERIFY(drawer);QTRY_VERIFY(drawer->property("position").toReal()>0.99);
        QVERIFY(backend->request("import_art",{{"path",folder->filePath("invalid.txt")}})["ok"].toBool());
        QTRY_VERIFY(!backend->state()["busy"].toBool());
        auto *notice=item("noticeBanner");QVERIFY(notice);QTRY_VERIFY(notice->isVisible());
        QVERIFY(!backend->state()["error"].toString().isEmpty());
        QVERIFY(notice->z()>drawer->property("z").toReal());
        capture("import-error");QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!notice->isVisible());
        QVERIFY(drawer->property("visible").toBool());
    }
    void importedWebMPreviewRetainsAlphaAndRespectsReducedMotion() {
#ifdef Q_OS_WIN
        const auto filename=QStringLiteral("../tools/ffmpeg.exe");
#else
        const auto filename=QStringLiteral("../tools/ffmpeg");
#endif
        auto ffmpeg = QDir(QFileInfo(QStringLiteral(__FILE__)).absolutePath()).filePath(filename);
        if (!QFileInfo::exists(ffmpeg)) ffmpeg=QStandardPaths::findExecutable("ffmpeg");
        if (ffmpeg.isEmpty()) QSKIP("FFmpeg is unavailable for the WebM fixture");
        const auto png=folder->filePath("alpha.png");
        QImage artwork(32,32,QImage::Format_RGBA8888);artwork.fill(Qt::transparent);
        for (int y=8;y<24;++y) for(int x=8;x<24;++x) artwork.setPixelColor(x,y,Qt::red);
        QVERIFY(artwork.save(png));
        const auto webm=folder->filePath("alpha.webm");
        QProcess encode;encode.start(ffmpeg,{"-v","error","-y","-loop","1","-i",png,"-t","0.3","-r","10","-c:v","libvpx-vp9","-pix_fmt","yuva420p","-an",webm});
        QVERIFY(encode.waitForFinished(15000));QCOMPARE(encode.exitCode(),0);
        QVERIFY(backend->request("import_art",{{"path",webm}})["ok"].toBool());
        capture("importing",0);
        QTRY_VERIFY_WITH_TIMEOUT(!backend->state()["busy"].toBool(),10000);
        QVERIFY2(backend->state()["error"].toString().isEmpty(),qPrintable(backend->state()["error"].toString()));
        const auto character=document()["characters"].toList().first().toMap();
        const auto asset=character["states"].toMap()["idle"].toMap()["asset"].toString();
        const auto preview=backend->state()["assetRoot"].toString()+"/"+asset+".preview.webp";
        QImageReader reader(preview);QVERIFY(reader.supportsAnimation());
        const auto decoded=reader.read();QVERIFY(!decoded.isNull());
        QCOMPARE(decoded.pixelColor(0,0).alpha(),0);
        QVERIFY(decoded.pixelColor(16,16).red()>180);
        QVERIFY(QFileInfo::exists(backend->state()["assetRoot"].toString()+"/"+asset+".webm"));
        QVERIFY(!evaluate(QString("window.showCharacter('%1')").arg(character["id"].toString())).isError());
        auto *drawer=window->findChild<QObject *>("characterEditor");QVERIFY(drawer);QTRY_VERIFY(drawer->property("position").toReal()>0.99);
        QQuickItem *animation=nullptr;
        QTRY_VERIFY((animation=drawer->findChild<QQuickItem *>("avatarAnimation")));
        QTRY_VERIFY(animation->property("frameCount").toInt()>1);
        QVERIFY(animation->property("playing").toBool());
        QVERIFY(!evaluate("window.edit(d => d.settings.reduced_motion = true)").isError());
        QTRY_VERIFY(!animation->property("playing").toBool());
        capture("webm-alpha");

    }
    void canvasFitsContentByDefaultAndFixedSizeRemainsOptional() {
        QVERIFY(document()["profiles"].toList().first().toMap()["groups"].toList().first().toMap()["auto_size"].toBool());
        QCOMPARE(publishedOutput()["width"].toInt(),64);
        for(const auto &id: {QString("123456789012345678"),QString("123456789012345679")})
            QVERIFY(backend->request("add_guest",{{"group",groupId},{"user",id},{"name","Guest"}})["ok"].toBool());
        QCOMPARE(backend->state()["outputs"].toMap()[groupId].toMap()["width"].toInt(),544);
        QCOMPARE(publishedOutput()["width"].toInt(),64);
        click(item("applyChanges"));QTRY_COMPARE(publishedOutput()["width"].toInt(),544);
        click(item("editComposition"));
        auto *automatic=item("autoCanvas");QVERIFY(automatic);QTRY_VERIFY(automatic->isVisible());QTest::qWait(100);
        click(automatic);
        QTRY_VERIFY(!document()["profiles"].toList().first().toMap()["groups"].toList().first().toMap()["auto_size"].toBool());
        QCOMPARE(document()["profiles"].toList().first().toMap()["groups"].toList().first().toMap()["width"].toInt(),544);
        click(automatic);
        QTRY_VERIFY(document()["profiles"].toList().first().toMap()["groups"].toList().first().toMap()["auto_size"].toBool());
        capture("automatic-canvas");
        window->resize(320,640);capture("automatic-canvas-narrow");
    }
};
QTEST_MAIN(UiTests)
#include "tests.moc"
