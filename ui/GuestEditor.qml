pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Drawer {
    id: editor
    objectName: "guestEditor"
    required property var app
    property var person: app.document.people[app.guestId] || ({
            name: "",
            character: null
        })
    property var member: app.group.members.find(m => m.user === app.guestId) || ({})
    property var character: app.document.characters.find(c => c.id === (member.character || person.character)) || null
    property var presence: app.backend.state.runtime.users[app.guestId] || ({})
    edge: Qt.RightEdge
    width: Math.min(app.width - 12, 380)
    height: app.height
    modal: true
    dim: true
    background: Rectangle {
        color: Theme.surface
        border.color: Theme.line
    }
    onOpened: forceActiveFocus()
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16
        RowLayout {
            Layout.fillWidth: true
            Text {
                text: editor.app.t("Invitado", "Guest")
                color: Theme.text
                font.pixelSize: 23
                font.weight: Font.DemiBold
                Layout.fillWidth: true
            }
            ActionButton {
                text: "×"
                Accessible.name: editor.app.t("Cerrar detalles", "Close details")
                quiet: true
                onClicked: editor.close()
            }
        }
        FocusScroll {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                Text {
                    text: editor.app.t("Nombre visible", "Display name")
                    color: Theme.muted
                }
                Field {
                    Layout.fillWidth: true
                    objectName: "guestName"
                    text: editor.person.name
                    labelText: editor.app.t("Nombre visible", "Display name")
                    onEditingFinished: editor.app.editPerson(editor.app.guestId, p => p.name = text)
                }
                Text {
                    text: editor.app.t("Personaje habitual", "Usual character")
                    color: Theme.muted
                }
                ComboBox {
                    Layout.fillWidth: true
                    model: [
                        {
                            id: null,
                            name: editor.app.t("Avatar de Discord", "Discord avatar")
                        }
                    ].concat(editor.app.document.characters)
                    textRole: "name"
                    valueRole: "id"
                    currentIndex: model.findIndex(c => c.id === editor.person.character)
                    Accessible.name: editor.app.t("Personaje habitual", "Usual character")
                    onActivated: editor.app.editPerson(editor.app.guestId, p => p.character = currentValue || null)
                }
                Text {
                    text: editor.app.t("En este grupo", "In this group")
                    color: Theme.muted
                }
                ComboBox {
                    Layout.fillWidth: true
                    model: [
                        {
                            id: null,
                            name: editor.app.t("Usar personaje habitual", "Use usual character")
                        }
                    ].concat(editor.app.document.characters)
                    textRole: "name"
                    valueRole: "id"
                    currentIndex: model.findIndex(c => c.id === (editor.member.character || null))
                    Accessible.name: editor.app.t("Personaje para este grupo", "Character for this group")
                    onActivated: editor.app.editMember(editor.app.guestId, m => m.character = currentValue || null)
                }
                Check {
                    Layout.fillWidth: true
                    text: editor.app.t("Incluir en la escena", "Include in scene")
                    checked: editor.member.enabled || false
                    onToggled: editor.app.editMember(editor.app.guestId, m => m.enabled = checked)
                }
                Check {
                    Layout.fillWidth: true
                    text: editor.app.t("Voltear imagen", "Mirror image")
                    checked: editor.member.mirror || false
                    onToggled: editor.app.editMember(editor.app.guestId, m => m.mirror = checked)
                }
                Check {
                    Layout.fillWidth: true
                    text: editor.app.t("Bloquear posición", "Lock position")
                    checked: editor.member.locked || false
                    onToggled: editor.app.editMember(editor.app.guestId, m => m.locked = checked)
                }
                Text {
                    text: editor.app.t("Tamaño", "Size")
                    color: Theme.muted
                }
                Slider {
                    Layout.fillWidth: true
                    from: 16
                    to: 1000
                    value: editor.member.size || 240
                    Accessible.name: editor.app.t("Tamaño del invitado", "Guest size")
                    onMoved: editor.app.editMember(editor.app.guestId, m => m.size = Math.round(value))
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    visible: editor.app.group.layout === "free"
                    Text {
                        text: "X"
                        color: Theme.muted
                    }
                    Field {
                        Layout.fillWidth: true
                        text: Math.round(editor.member.x || 0)
                        labelText: editor.app.t("Posición horizontal", "Horizontal position")
                        validator: DoubleValidator {
                            bottom: -8192
                            top: 8192
                        }
                        onEditingFinished: editor.app.editMember(editor.app.guestId, m => m.x = Number(text))
                    }
                    Text {
                        text: "Y"
                        color: Theme.muted
                    }
                    Field {
                        Layout.fillWidth: true
                        text: Math.round(editor.member.y || 0)
                        labelText: editor.app.t("Posición vertical", "Vertical position")
                        validator: DoubleValidator {
                            bottom: -8192
                            top: 8192
                        }
                        onEditingFinished: editor.app.editMember(editor.app.guestId, m => m.y = Number(text))
                    }
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    visible: editor.app.group.layout === "free"
                    ActionButton {
                        text: editor.app.t("Centrar", "Center")
                        onClicked: editor.app.editMember(editor.app.guestId, m => {
                            m.x = (editor.app.output.width - m.size) / 2 + editor.app.output.origin_x;
                            m.y = (editor.app.output.height - m.size) / 2 + editor.app.output.origin_y;
                        })
                    }
                    ActionButton {
                        text: editor.app.t("Alinear abajo", "Align bottom")
                        onClicked: editor.app.editMember(editor.app.guestId, m => m.y = Math.max(0, editor.app.output.height - m.size - 40) + editor.app.output.origin_y)
                    }
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: editor.app.t("Traer al frente", "Bring forward")
                        onClicked: editor.app.editGroup(g => {
                            const i = g.members.findIndex(m => m.user === editor.app.guestId);
                            if (i >= 0 && i + 1 < g.members.length) {
                                const m = g.members.splice(i, 1)[0];
                                g.members.splice(i + 1, 0, m);
                            }
                        })
                    }
                    ActionButton {
                        text: editor.app.t("Enviar atrás", "Send backward")
                        onClicked: editor.app.editGroup(g => {
                            const i = g.members.findIndex(m => m.user === editor.app.guestId);
                            if (i > 0) {
                                const m = g.members.splice(i, 1)[0];
                                g.members.splice(i - 1, 0, m);
                            }
                        })
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.line
                    Layout.topMargin: 12
                    Layout.bottomMargin: 10
                }
                Text {
                    text: editor.app.t("Controles del directo", "Live controls")
                    color: Theme.text
                    font.weight: Font.DemiBold
                    font.pixelSize: 16
                }
                Text {
                    text: editor.app.t("Estos cambios se ven al instante.", "These changes appear immediately.")
                    color: Theme.muted
                    font.pixelSize: 12
                }
                ActionButton {
                    text: editor.presence.hidden ? editor.app.t("Mostrar en el directo", "Show on stream") : editor.app.t("Ocultar del directo", "Hide from stream")
                    Layout.fillWidth: true
                    onClicked: editor.app.backend.request("live", {
                        user: editor.app.guestId,
                        action: "toggle"
                    })
                }
                ComboBox {
                    Layout.fillWidth: true
                    model: [editor.app.t("Automática", "Automatic")].concat(editor.character ? Object.keys(editor.character.expressions) : [])
                    currentIndex: editor.presence.expression ? model.indexOf(editor.presence.expression) : 0
                    Accessible.name: editor.app.t("Expresión en directo", "Live expression")
                    onActivated: editor.app.backend.request("live", {
                        user: editor.app.guestId,
                        action: currentIndex === 0 ? "reset" : currentText
                    })
                }
                ActionButton {
                    text: editor.app.t("Copiar URL individual", "Copy individual URL")
                    Layout.fillWidth: true
                    quiet: true
                    onClicked: editor.app.copyUrl("person", editor.app.guestId)
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.line
                    Layout.topMargin: 12
                    Layout.bottomMargin: 10
                }
                Text {
                    text: editor.app.t("Probar en la vista previa", "Test in preview")
                    color: Theme.text
                    font.weight: Font.Medium
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 6
                    Repeater {
                        model: [
                            {
                                id: "idle",
                                es: "Reposo",
                                en: "Idle"
                            },
                            {
                                id: "speaking",
                                es: "Habla",
                                en: "Speaking"
                            },
                            {
                                id: "muted",
                                es: "Mute",
                                en: "Muted"
                            },
                            {
                                id: "absent",
                                es: "Ausente",
                                en: "Absent"
                            },
                            {
                                id: "off",
                                es: "Terminar",
                                en: "Stop"
                            }
                        ]
                        ActionButton {
                            required property var modelData
                            text: editor.app.es ? modelData.es : modelData.en
                            quiet: true
                            onClicked: editor.app.backend.request("test", {
                                user: editor.app.guestId,
                                state: modelData.id
                            })
                        }
                    }
                }
                Text {
                    text: editor.app.t("La prueba no aparece en el stream.", "The test does not appear on stream.")
                    color: Theme.muted
                    font.pixelSize: 12
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
                ActionButton {
                    text: editor.app.t("Quitar del grupo", "Remove from group")
                    destructive: true
                    quiet: true
                    Layout.fillWidth: true
                    onClicked: {
                        editor.app.editGroup(g => g.members = g.members.filter(m => m.user !== editor.app.guestId));
                        editor.close();
                    }
                }
            }
        }
        ActionButton {
            objectName: "closeGuest"
            text: editor.app.t("Listo", "Done")
            primary: true
            Layout.fillWidth: true
            onClicked: editor.close()
        }
    }
}
