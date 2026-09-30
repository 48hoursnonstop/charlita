pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Item {
    id: library
    required property var app
    ColumnLayout {
        anchors.fill: parent
        spacing: 20
        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 6
                Text {
                    text: library.app.t("Personajes con personalidad.", "Characters with character.")
                    color: Theme.text
                    font.family: Theme.family
                    font.pixelSize: library.width < 650 ? 25 : 32
                    font.weight: Font.DemiBold
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                Text {
                    text: library.app.t("Tu colección, lista para cualquier llamada.", "Your collection, ready for any call.")
                    color: Theme.muted
                    font.pixelSize: 14
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
            }
            ActionButton {
                text: library.app.t("+ Importar", "+ Import")
                primary: true
                enabled: !library.app.busy
                onClicked: importMenu.open()
            }
            Menu {
                id: importMenu
                MenuItem {
                    text: library.app.t("Imagen o animación…", "Image or animation…")
                    onTriggered: library.app.backend.pickFile("import_art")
                }
                MenuItem {
                    text: library.app.t("Paquete de personaje…", "Character package…")
                    onTriggered: library.app.backend.pickFile("import_character")
                }
            }
        }
        Text {
            text: "PNG · JPEG · WebP · GIF · sprites · WebM"
            color: Theme.muted
            font.pixelSize: 12
        }
        FocusScroll {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            Flow {
                width: parent.width
                spacing: 16
                Repeater {
                    model: library.app.document.characters
                    Rectangle {
                        id: card
                        required property var modelData
                        property var pose: modelData.states.idle || null
                        property var asset: pose ? library.app.document.assets[pose.asset] : null
                        width: Math.max(150, (library.width - Math.floor(library.width / 225) * 16) / Math.max(1, Math.floor(library.width / 225)))
                        height: 240
                        radius: 16
                        color: Theme.surface
                        border.color: hover.hovered ? "#52644f" : Theme.line
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 18
                            spacing: 10
                            Item {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Avatar {
                                    anchors.centerIn: parent
                                    width: Math.min(parent.width, parent.height)
                                    height: width
                                    app: library.app
                                    pose: card.pose
                                    media: card.asset ? ({
                                            kind: card.asset.extension,
                                            width: card.asset.width,
                                            height: card.asset.height,
                                            url: ""
                                        }) : null
                                    label: card.modelData.name
                                    playing: false
                                }
                            }
                            Text {
                                text: card.modelData.name
                                color: Theme.text
                                font.pixelSize: 15
                                font.weight: Font.Medium
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                ToolTip.visible: hover.hovered && truncated
                                ToolTip.text: text
                            }
                            Text {
                                text: Object.keys(card.modelData.expressions).length + library.app.t(" expresiones", " expressions")
                                color: Theme.muted
                                font.pixelSize: 12
                            }
                        }
                        HoverHandler {
                            id: hover
                        }
                        Button {
                            anchors.fill: parent
                            Accessible.name: library.app.t("Editar personaje ", "Edit character ") + card.modelData.name
                            contentItem: Item {}
                            background: Rectangle {
                                color: "transparent"
                                radius: 16
                                border.width: parent.activeFocus ? 2 : 0
                                border.color: Theme.accent
                            }
                            onClicked: library.app.showCharacter(card.modelData.id)
                        }
                    }
                }
            }
        }
    }
    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 40, 440)
        spacing: 12
        visible: library.app.document.characters.length === 0
        Image {
            source: "logo.svg"
            Layout.preferredWidth: 64
            Layout.preferredHeight: 64
            Layout.alignment: Qt.AlignHCenter
            opacity: 0.7
        }
        Text {
            text: library.app.t("Aquí empieza tu elenco", "Your cast starts here")
            color: Theme.text
            font.pixelSize: 24
            font.weight: Font.DemiBold
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
        }
        Text {
            text: library.app.t("Importa el arte de tus invitados o un paquete que te hayan enviado. Después podrás asignarlo a cualquier grupo.", "Import your guests' artwork or a package they've sent you. Then assign it to any group.")
            color: Theme.muted
            font.pixelSize: 14
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
        }
        ActionButton {
            text: library.app.t("Importar personaje", "Import character")
            primary: true
            Layout.alignment: Qt.AlignHCenter
            onClicked: importMenu.open()
        }
    }
}
