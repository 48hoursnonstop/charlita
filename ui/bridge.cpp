#include "bridge.h"
#include <QApplication>
#include <QClipboard>
#include <QDesktopServices>
#include <QFileDialog>
#include <QFontDatabase>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSystemTrayIcon>
#include <QUrl>
extern "C" {
void *charlita_create(const char *options);
char *charlita_last_error();
char *charlita_request(void *handle, const char *request);
void charlita_set_wake(void *handle, void *context, void (*callback)(void *));
void charlita_destroy(void *handle);
void charlita_string_free(char *string);
}
static QString takeString(char *pointer) {
    const auto text = QString::fromUtf8(pointer);
    charlita_string_free(pointer);
    return text;
}
Bridge::Bridge(const QVariantMap &options, QObject *parent) : QObject(parent), m_hasTray(QSystemTrayIcon::isSystemTrayAvailable()) {
    const auto fontId = QFontDatabase::addApplicationFont(QStringLiteral(":/qt/qml/" CHARLITA_QML_URI "/ui/fonts/InterVariable.ttf"));
    const auto families = QFontDatabase::applicationFontFamilies(fontId);
    if (!families.isEmpty()) { QFont font(families.first()); font.setPixelSize(14); QGuiApplication::setFont(font); }
    const auto json = QJsonDocument(QJsonObject::fromVariantMap(options)).toJson(QJsonDocument::Compact);
    m_handle = charlita_create(json.constData());
    if (!m_handle) { m_error = takeString(charlita_last_error()); return; }
    charlita_set_wake(m_handle, this, &Bridge::wake);
    refresh();
    const auto settings = m_state.value("document").toMap().value("settings").toMap();
    if (!settings.value("hotkeys").toList().isEmpty()) request("register_hotkeys");
}
Bridge::~Bridge() { if (m_handle) charlita_destroy(m_handle); }
void Bridge::wake(void *context) {
    auto *bridge = static_cast<Bridge *>(context);
    if (bridge->m_queued.exchange(true)) return;
    QMetaObject::invokeMethod(bridge, [bridge] { bridge->m_queued = false; bridge->refresh(); }, Qt::QueuedConnection);
}
QVariantMap Bridge::call(const QString &method, const QVariantMap &args) {
    const auto json = QJsonDocument(QJsonObject::fromVariantMap({{"method", method}, {"args", args}})).toJson(QJsonDocument::Compact);
    const auto response = takeString(charlita_request(m_handle, json.constData()));
    QJsonParseError error;
    const auto result = QJsonDocument::fromJson(response.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError) return {{"ok", false}, {"error", "Cannot read engine response / No se pudo leer la respuesta"}};
    return result.object().toVariantMap();
}
QVariantMap Bridge::request(const QString &method, const QVariantMap &args) {
    const auto result = call(method, args);
    if (!result.value("ok").toBool()) { m_error = result.value("error").toString(); emit errorChanged(); }
    refresh();
    return result;
}
void Bridge::refresh() {
    const auto result = call("snapshot", {});
    if (!result.value("ok").toBool()) { m_error = result.value("error").toString(); emit errorChanged(); return; }
    m_state = result.value("value").toMap();
    const auto commands = m_state.value("commands").toList();
    emit stateChanged();
    for (const auto &command : commands) {
        if (command.toString() == "Show") emit showRequested();
        else if (command.toString() == "Quit") quit();
        else if (command.toString() == "RegisterHotkeys") request("register_hotkeys");
    }
}
void Bridge::pickFile(const QString &method, const QString &character, const QString &pose) {
    const bool save = method.startsWith("export") || method == "diagnostics";
    const QString filters = method == "import_art" ? "Artwork (*.png *.jpg *.jpeg *.webp *.gif *.webm)"
        : method == "diagnostics" ? "Diagnostics (*.json)" : "Charlita (*.charlita)";
    QString path = save ? QFileDialog::getSaveFileName(nullptr, "Charlita", method == "diagnostics" ? "charlita-diagnostics.json" : "character.charlita", filters)
        : QFileDialog::getOpenFileName(nullptr, "Charlita", {}, filters);
    if (path.isEmpty()) return;
    if (save && method != "diagnostics" && !path.endsWith(".charlita", Qt::CaseInsensitive)) path += ".charlita";
    request(method, {{"path", path}, {"character", character}, {"state", pose}});
}
void Bridge::copy(const QString &text) { QGuiApplication::clipboard()->setText(text); }
void Bridge::open(const QString &url) {
    QUrl target(url);
    if (target.scheme().isEmpty()) target = QUrl::fromLocalFile(url);
    if (!QDesktopServices::openUrl(target)) { m_error = "Cannot open the link. Copy it and open it manually / Copia el enlace y ábrelo manualmente"; emit errorChanged(); }
}
QString Bridge::localFile(const QString &path) const { return QUrl::fromLocalFile(path).toString(); }
void Bridge::dismiss() { m_error.clear(); emit errorChanged(); request("dismiss"); }
void Bridge::quit() { QCoreApplication::quit(); }
