#include "bridge.h"
#include <QApplication>
#include <QCommandLineParser>
#include <QMenu>
#include <QMessageBox>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QSystemTrayIcon>
#include <QTimer>
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    app.setApplicationName("Charlita"); app.setApplicationVersion("0.1.0");
    app.setOrganizationName("48hoursnonstop");
    app.setDesktopFileName("io.github.48hoursnonstop.charlita");
    QQuickStyle::setStyle("Basic");
    QCommandLineParser parser; parser.addHelpOption(); parser.addVersionOption();
    parser.addOption({"portable", "Store settings alongside the application"});
    parser.addOption({"data-dir", "Use a separate data directory", "path"});
    parser.addOption({"port", "Local overlay port", "number"});
    parser.addOption({"screenshot", "Save the native window and exit (UI verification)", "path"});
    parser.process(app);
    QVariantMap options{{"portable", parser.isSet("portable")}, {"data_dir", parser.value("data-dir")}};
    if (parser.isSet("port")) { bool ok = false; const auto port = parser.value("port").toUInt(&ok); if (!ok || port < 1024 || port > 65535) return 2; options["port"] = port; }
    Bridge backend(options);
    if (!backend.ready()) {
        if (backend.error() == "already_running") return 0;
        QMessageBox::critical(nullptr, "Charlita", backend.error()); return 1;
    }
    app.setQuitOnLastWindowClosed(!backend.hasTray());
    QQmlApplicationEngine engine;
    engine.setInitialProperties({{"backend", QVariant::fromValue(&backend)}});
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreationFailed, &app, [] { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.loadFromModule("Charlita", "Main");
    if (engine.rootObjects().isEmpty()) return 1;
    auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
    QObject::connect(&backend, &Bridge::showRequested, window, [window] { window->show(); window->raise(); window->requestActivate(); });
    QSystemTrayIcon tray(QIcon(":/qt/qml/Charlita/ui/logo.svg"));
    QMenu menu;
    const bool es = backend.state().value("document").toMap().value("settings").toMap().value("language").toString() == "es";
    menu.addAction(es ? "Abrir Charlita" : "Open Charlita", window, [window] { window->show(); window->raise(); window->requestActivate(); });
    menu.addSeparator(); menu.addAction(es ? "Salir" : "Quit", &app, &QCoreApplication::quit);
    tray.setContextMenu(&menu); tray.setToolTip("Charlita");
    QObject::connect(&tray, &QSystemTrayIcon::activated, window, [window](QSystemTrayIcon::ActivationReason reason) { if (reason == QSystemTrayIcon::Trigger || reason == QSystemTrayIcon::DoubleClick) { window->show(); window->raise(); window->requestActivate(); } });
    if (backend.hasTray()) tray.show();
    if (parser.isSet("screenshot")) QTimer::singleShot(1800, window, [window, &app, &parser] { if (!window->grabWindow().save(parser.value("screenshot"))) QCoreApplication::exit(3); else app.quit(); });
    return app.exec();
}
