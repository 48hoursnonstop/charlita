#pragma once
#include <QObject>
#include <QtQml/qqmlregistration.h>
#include <QVariantMap>
#include <atomic>
class Bridge final : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Provided by the application")
    Q_PROPERTY(QVariantMap state READ state NOTIFY stateChanged)
    Q_PROPERTY(QString error READ error NOTIFY errorChanged)
    Q_PROPERTY(bool hasTray READ hasTray CONSTANT)
public:
    explicit Bridge(const QVariantMap &options, QObject *parent = nullptr);
    ~Bridge() override;
    bool ready() const { return m_handle != nullptr; }
    QVariantMap state() const { return m_state; }
    QString error() const { return m_error; }
    bool hasTray() const { return m_hasTray; }
    Q_INVOKABLE QVariantMap request(const QString &method, const QVariantMap &args = {});
    Q_INVOKABLE void pickFile(const QString &method, const QString &character = {}, const QString &pose = "idle");
    Q_INVOKABLE void copy(const QString &text);
    Q_INVOKABLE void open(const QString &url);
    Q_INVOKABLE QString localFile(const QString &path) const;
    Q_INVOKABLE void dismiss();
    Q_INVOKABLE void quit();
    void refresh();
signals:
    void stateChanged();
    void errorChanged();
    void showRequested();
private:
    static void wake(void *context);
    QVariantMap call(const QString &method, const QVariantMap &args);
    void *m_handle = nullptr;
    QVariantMap m_state;
    QString m_error;
    bool m_hasTray = false;
    std::atomic_bool m_queued = false;
};
