pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

FocusScroll {
    id: settings
    required property var app
    property var backend: app.backend
    property var preferences: app.document.settings
    clip: true
    function editSettings(change) {
        app.edit(d => change(d.settings));
    }
    ColumnLayout {
        width: Math.min(settings.availableWidth, 760)
        spacing: 24
        Text {
            text: settings.app.t("A tu manera.", "Make it yours.")
            color: Theme.text
            font.pixelSize: 32
            font.weight: Font.DemiBold
        }
        Text {
            text: settings.app.t("Conexión, comodidad y lo que pasa detrás de escena.", "Connection, comfort and what happens behind the scenes.")
            color: Theme.muted
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font.pixelSize: 14
        }
        GroupBox {
            title: settings.app.t("Comodidad", "Comfort")
            Layout.fillWidth: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: settings.app.t("Idioma", "Language")
                        color: Theme.muted
                        Layout.fillWidth: true
                    }
                    ComboBox {
                        model: ["Español", "English"]
                        currentIndex: settings.preferences.language === "es" ? 0 : 1
                        Accessible.name: settings.app.t("Idioma", "Language")
                        onActivated: settings.editSettings(s => s.language = currentIndex === 0 ? "es" : "en")
                    }
                }
                Check {
                    Layout.fillWidth: true
                    text: settings.app.t("Reducir movimiento y pausar animaciones", "Reduce motion and pause animations")
                    checked: settings.preferences.reduced_motion
                    onToggled: {
                        settings.editSettings(s => s.reduced_motion = checked);
                        settings.backend.request("settings");
                    }
                }
            }
        }
        GroupBox {
            title: "Discord"
            Layout.fillWidth: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                Text {
                    text: settings.app.t("Conecta el cliente oficial de escritorio. Charlita seguirá tu llamada actual.", "Connect the official desktop client. Charlita follows your current call.")
                    color: Theme.muted
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Text {
                    text: settings.app.t("ID público de la aplicación", "Public application ID")
                    color: Theme.muted
                }
                Field {
                    Layout.fillWidth: true
                    text: settings.preferences.discord_app
                    labelText: settings.app.t("ID público de la aplicación", "Public application ID")
                    onEditingFinished: settings.editSettings(s => s.discord_app = text)
                }
                Text {
                    text: settings.app.t("La aplicación debe tener Public Client y permisos RPC de voz habilitados en Discord.", "The application needs Public Client and RPC voice permissions enabled in Discord.")
                    color: Theme.muted
                    font.pixelSize: 12
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Text {
                    text: settings.backend.state.redirect
                    color: Theme.muted
                    font.pixelSize: 12
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: settings.app.t("Autorizar Discord", "Authorize Discord")
                        primary: true
                        onClicked: {
                            settings.backend.request("settings");
                            settings.backend.request("command", {
                                name: "authorize"
                            });
                        }
                    }
                    ActionButton {
                        text: settings.app.t("Reconectar", "Reconnect")
                        onClicked: {
                            settings.backend.request("settings");
                            settings.backend.request("command", {
                                name: "connect"
                            });
                        }
                    }
                    ActionButton {
                        text: settings.app.t("Cerrar sesión", "Sign out")
                        quiet: true
                        onClicked: settings.backend.request("command", {
                            name: "disconnect"
                        })
                    }
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: settings.app.t("Portal de desarrolladores", "Developer portal")
                        quiet: true
                        onClicked: settings.backend.open("https://discord.com/developers/applications")
                    }
                    ActionButton {
                        text: settings.app.t("Copiar redirección", "Copy redirect")
                        quiet: true
                        onClicked: settings.backend.copy(settings.backend.state.redirect)
                    }
                }
                Check {
                    Layout.fillWidth: true
                    text: settings.app.t("Fijar el canal actual", "Pin current channel")
                    enabled: !!settings.backend.state.runtime.channel || !!settings.preferences.pinned_channel
                    checked: !!settings.preferences.pinned_channel
                    onToggled: {
                        settings.editSettings(s => s.pinned_channel = checked ? settings.backend.state.runtime.channel : null);
                        settings.backend.request("settings");
                    }
                }
            }
        }
        GroupBox {
            title: settings.app.t("Atajos del directo", "Live hotkeys")
            Layout.fillWidth: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                Text {
                    text: settings.app.t("Muestra, oculta o cambia una expresión sin salir de tu juego.", "Show, hide or change an expression without leaving your game.")
                    color: Theme.muted
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Repeater {
                    model: settings.preferences.hotkeys
                    ColumnLayout {
                        id: binding
                        required property var modelData
                        Layout.fillWidth: true
                        function update(change) {
                            settings.editSettings(s => {
                                const b = s.hotkeys.find(b => b.id === modelData.id);
                                if (b)
                                    change(b);
                            });
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Field {
                                Layout.fillWidth: true
                                text: binding.modelData.shortcut
                                labelText: settings.app.t("Teclas del atajo", "Shortcut keys")
                                onEditingFinished: binding.update(b => b.shortcut = text)
                            }
                            ActionButton {
                                text: "×"
                                Accessible.name: settings.app.t("Quitar atajo", "Remove hotkey")
                                quiet: true
                                onClicked: settings.editSettings(s => s.hotkeys = s.hotkeys.filter(b => b.id !== binding.modelData.id))
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            ComboBox {
                                Layout.fillWidth: true
                                model: Object.values(settings.app.document.people)
                                textRole: "name"
                                valueRole: "id"
                                currentIndex: model.findIndex(p => p.id === binding.modelData.user)
                                Accessible.name: settings.app.t("Invitado del atajo", "Hotkey guest")
                                onActivated: binding.update(b => b.user = currentValue)
                            }
                            Field {
                                Layout.fillWidth: true
                                text: binding.modelData.action
                                labelText: settings.app.t("Acción o expresión", "Action or expression")
                                onEditingFinished: binding.update(b => b.action = text)
                            }
                        }
                    }
                }
                Text {
                    text: settings.app.t("Acciones: toggle, show, hide, reset o el nombre de una expresión.", "Actions: toggle, show, hide, reset or an expression name.")
                    color: Theme.muted
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    font.pixelSize: 12
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: settings.app.t("+ Añadir atajo", "+ Add hotkey")
                        enabled: Object.keys(settings.app.document.people).length > 0
                        onClicked: settings.editSettings(s => s.hotkeys.push({
                                id: "key-" + Date.now(),
                                shortcut: "Ctrl+Alt+1",
                                user: Object.keys(settings.app.document.people)[0],
                                action: "toggle"
                            }))
                    }
                    ActionButton {
                        text: settings.app.t("Registrar atajos", "Register hotkeys")
                        onClicked: {
                            settings.backend.request("settings");
                            settings.backend.request("register_hotkeys");
                        }
                    }
                }
            }
        }
        GroupBox {
            title: settings.app.t("Actualizaciones", "Updates")
            Layout.fillWidth: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                Field {
                    Layout.fillWidth: true
                    text: settings.preferences.update_repo
                    labelText: settings.app.t("Repositorio de actualizaciones: propietario/repo", "Update repository: owner/repo")
                    onEditingFinished: settings.editSettings(s => s.update_repo = text)
                }
                Check {
                    Layout.fillWidth: true
                    text: settings.app.t("Buscar al iniciar", "Check on startup")
                    checked: settings.preferences.auto_check
                    onToggled: settings.editSettings(s => s.auto_check = checked)
                }
                Text {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: Theme.muted
                    text: {
                        const s = settings.backend.state.update.status;
                        if (s === "checking")
                            return settings.app.t("Buscando actualizaciones…", "Checking for updates…");
                        if (s === "current")
                            return settings.app.t("Tienes la última versión", "You're up to date");
                        if (s === "no_releases")
                            return settings.app.t("Todavía no hay versiones publicadas", "No releases published yet");
                        if (s === "available")
                            return settings.app.t("Hay una actualización disponible", "An update is available");
                        if (s === "downloading")
                            return settings.app.t("Descargando y verificando…", "Downloading and verifying…");
                        if (s === "ready")
                            return settings.app.t("Descarga verificada. Puedes abrirla cuando quieras.", "Download verified. Open it when you're ready.");
                        return s || "";
                    }
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: settings.app.t("Buscar actualización", "Check for updates")
                        onClicked: {
                            settings.backend.request("settings");
                            settings.backend.request("command", {
                                name: "check_update"
                            });
                        }
                    }
                    ActionButton {
                        text: settings.app.t("Descargar", "Download")
                        visible: settings.backend.state.update.status === "available"
                        onClicked: settings.backend.request("command", {
                            name: "install_update"
                        })
                    }
                    ActionButton {
                        text: settings.app.t("Abrir descarga", "Open download")
                        visible: !!settings.backend.state.update.downloaded
                        onClicked: settings.backend.open(settings.backend.state.update.downloaded)
                    }
                    ActionButton {
                        text: settings.app.t("Ver en GitHub", "View on GitHub")
                        quiet: true
                        visible: !!settings.backend.state.update.url
                        onClicked: settings.backend.open(settings.backend.state.update.url)
                    }
                }
            }
        }
        GroupBox {
            title: settings.app.t("Salida y diagnóstico", "Output and diagnostics")
            Layout.fillWidth: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: settings.app.t("Puerto local", "Local port")
                        color: Theme.muted
                        Layout.fillWidth: true
                    }
                    SpinBox {
                        from: 1024
                        to: 65535
                        value: settings.preferences.port
                        editable: true
                        Accessible.name: settings.app.t("Puerto local", "Local port")
                        onValueModified: settings.editSettings(s => s.port = value)
                    }
                }
                Text {
                    text: settings.app.t("Cambiar el puerto requiere reiniciar Charlita.", "Changing the port requires restarting Charlita.")
                    color: Theme.muted
                    font.pixelSize: 12
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Text {
                    text: settings.backend.state.root
                    color: Theme.muted
                    font.pixelSize: 12
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: settings.app.t("Exportar diagnóstico…", "Export diagnostics…")
                        onClicked: settings.backend.pickFile("diagnostics")
                    }
                    ActionButton {
                        text: settings.app.t("Código fuente", "Source code")
                        quiet: true
                        onClicked: settings.backend.open("https://github.com/48hoursnonstop/charlita")
                    }
                }
            }
        }
        ActionButton {
            text: settings.app.t("Guardar ajustes", "Save settings")
            primary: true
            onClicked: settings.backend.request("settings")
        }
        Item {
            Layout.preferredHeight: 24
        }
    }
}
