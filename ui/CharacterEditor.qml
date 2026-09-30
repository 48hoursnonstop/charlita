pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Drawer {
    id: editor
    objectName: "characterEditor"
    required property var app
    property var character: app.document.characters.find(c => c.id === app.characterId) || ({
            name: "",
            states: {},
            expressions: {},
            effects: {
                brightness: 1.12,
                jump: 10,
                duration_ms: 180,
                release_ms: 160,
                glow: false,
                dim_idle: 1
            }
        })
    property string poseKey: "idle"
    property var pose: poseKey.startsWith("expression:") ? character.expressions[poseKey.slice(11)] : character.states[poseKey]
    property var asset: pose ? app.document.assets[pose.asset] : null
    property bool expression: poseKey.startsWith("expression:")
    function editPose(change) {
        app.editCharacter(app.characterId, c => {
            const p = expression ? c.expressions[poseKey.slice(11)] : c.states[poseKey];
            if (p)
                change(p);
        });
    }
    edge: Qt.RightEdge
    width: Math.min(app.width - 12, 460)
    height: app.height
    modal: true
    dim: true
    background: Rectangle {
        color: Theme.surface
        border.color: Theme.line
    }
    onOpened: {
        poseKey = "idle";
        forceActiveFocus();
    }
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16
        RowLayout {
            Layout.fillWidth: true
            Text {
                text: editor.app.t("Personaje", "Character")
                color: Theme.text
                font.pixelSize: 23
                font.weight: Font.DemiBold
                Layout.fillWidth: true
            }
            ActionButton {
                text: "×"
                Accessible.name: editor.app.t("Cerrar personaje", "Close character")
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
                Field {
                    Layout.fillWidth: true
                    text: editor.character.name
                    labelText: editor.app.t("Nombre del personaje", "Character name")
                    onEditingFinished: editor.app.editCharacter(editor.app.characterId, c => c.name = text)
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 200
                    radius: 12
                    color: Theme.background
                    Avatar {
                        anchors.centerIn: parent
                        width: 160
                        height: 160
                        app: editor.app
                        pose: editor.pose
                        media: editor.asset ? ({
                                url: "",
                                kind: editor.asset.extension,
                                width: editor.asset.width,
                                height: editor.asset.height
                            }) : null
                        label: editor.character.name
                        playing: editor.visible && editor.app.visible && editor.app.visibility !== Window.Minimized && !editor.app.document.settings.reduced_motion
                    }
                }
                Text {
                    text: editor.app.t("Estado o expresión", "State or expression")
                    color: Theme.muted
                }
                ComboBox {
                    Layout.fillWidth: true
                    property var entries: [
                        {
                            id: "idle",
                            name: editor.app.t("Reposo", "Idle")
                        },
                        {
                            id: "speaking",
                            name: editor.app.t("Hablando", "Speaking")
                        },
                        {
                            id: "muted",
                            name: editor.app.t("Silenciado", "Muted")
                        },
                        {
                            id: "absent",
                            name: editor.app.t("Ausente", "Absent")
                        }
                    ].concat(Object.keys(editor.character.expressions).map(n => ({
                                id: "expression:" + n,
                                name: n
                            })))
                    model: entries
                    textRole: "name"
                    valueRole: "id"
                    currentIndex: entries.findIndex(e => e.id === editor.poseKey)
                    Accessible.name: editor.app.t("Estado o expresión", "State or expression")
                    onActivated: editor.poseKey = currentValue
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: editor.app.t("Cambiar imagen…", "Change image…")
                        enabled: !editor.app.busy
                        onClicked: editor.app.backend.pickFile("import_art", editor.app.characterId, editor.poseKey)
                    }
                    ActionButton {
                        text: editor.app.t("Quitar imagen", "Remove image")
                        quiet: true
                        enabled: !!editor.pose
                        onClicked: editor.app.editCharacter(editor.app.characterId, c => {
                            if (editor.expression)
                                delete c.expressions[editor.poseKey.slice(11)];
                            else
                                delete c.states[editor.poseKey];
                        })
                    }
                }
                Text {
                    text: editor.asset ? editor.asset.name : editor.app.t("Usará la imagen de reposo o el avatar de Discord.", "Uses the idle image or Discord avatar.")
                    color: Theme.muted
                    font.pixelSize: 12
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                }
                RowLayout {
                    Layout.fillWidth: true
                    Field {
                        id: expressionName
                        Layout.fillWidth: true
                        placeholderText: editor.app.t("Nueva expresión", "New expression")
                    }
                    ActionButton {
                        text: "+"
                        Accessible.name: editor.app.t("Añadir expresión", "Add expression")
                        enabled: !!editor.character.states.idle
                        onClicked: {
                            const name = expressionName.text.trim();
                            if (name) {
                                editor.app.editCharacter(editor.app.characterId, c => c.expressions[name] = JSON.parse(JSON.stringify(c.states.idle)));
                                editor.poseKey = "expression:" + name;
                                expressionName.text = "";
                            }
                        }
                    }
                }
                Text {
                    text: editor.app.t("Recorte", "Crop")
                    color: Theme.text
                    font.weight: Font.DemiBold
                    visible: !!editor.pose
                    Layout.topMargin: 10
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    visible: !!editor.pose
                    Repeater {
                        model: [
                            {
                                index: 0,
                                es: "Izquierda",
                                en: "Left"
                            },
                            {
                                index: 1,
                                es: "Arriba",
                                en: "Top"
                            },
                            {
                                index: 2,
                                es: "Ancho",
                                en: "Width"
                            },
                            {
                                index: 3,
                                es: "Alto",
                                en: "Height"
                            }
                        ]
                        RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Text {
                                text: editor.app.es ? modelData.es : modelData.en
                                color: Theme.muted
                                Layout.fillWidth: true
                                font.pixelSize: 12
                            }
                            SpinBox {
                                from: modelData.index < 2 ? 0 : 1
                                to: modelData.index < 2 ? 99 : 100
                                value: editor.pose ? Math.round(editor.pose.crop[modelData.index] * 100) : 0
                                editable: true
                                Accessible.name: editor.app.t("Recorte: ", "Crop: ") + (editor.app.es ? modelData.es : modelData.en)
                                onValueModified: editor.editPose(p => {
                                    const index = modelData.index;
                                    p.crop[index] = value / 100;
                                    if (index < 2)
                                        p.crop[index + 2] = Math.max(0.01, Math.min(p.crop[index + 2], 1 - p.crop[index]));
                                    else
                                        p.crop[index] = Math.min(p.crop[index], 1 - p.crop[index - 2]);
                                })
                            }
                        }
                    }
                }
                Check {
                    Layout.fillWidth: true
                    visible: !!editor.pose
                    text: editor.app.t("Es una hoja de sprites", "Sprite sheet")
                    checked: editor.pose ? !!editor.pose.sprite : false
                    onToggled: editor.editPose(p => p.sprite = checked ? {
                            columns: 4,
                            rows: 1,
                            frames: 4,
                            fps: 12
                        } : null)
                }
                GridLayout {
                    visible: !!editor.pose && !!editor.pose.sprite
                    columns: 2
                    Layout.fillWidth: true
                    Text {
                        text: editor.app.t("Columnas", "Columns")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 1
                        to: 256
                        value: editor.pose && editor.pose.sprite ? editor.pose.sprite.columns : 1
                        editable: true
                        Accessible.name: editor.app.t("Columnas del sprite", "Sprite columns")
                        onValueModified: editor.editPose(p => {
                            p.sprite.columns = value;
                            p.sprite.frames = Math.min(p.sprite.frames, p.sprite.columns * p.sprite.rows);
                        })
                    }
                    Text {
                        text: editor.app.t("Filas", "Rows")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 1
                        to: 256
                        value: editor.pose && editor.pose.sprite ? editor.pose.sprite.rows : 1
                        editable: true
                        Accessible.name: editor.app.t("Filas del sprite", "Sprite rows")
                        onValueModified: editor.editPose(p => {
                            p.sprite.rows = value;
                            p.sprite.frames = Math.min(p.sprite.frames, p.sprite.columns * p.sprite.rows);
                        })
                    }
                    Text {
                        text: editor.app.t("Fotogramas", "Frames")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 1
                        to: editor.pose && editor.pose.sprite ? editor.pose.sprite.rows * editor.pose.sprite.columns : 1
                        value: editor.pose && editor.pose.sprite ? editor.pose.sprite.frames : 1
                        editable: true
                        Accessible.name: editor.app.t("Fotogramas del sprite", "Sprite frames")
                        onValueModified: editor.editPose(p => p.sprite.frames = value)
                    }
                    Text {
                        text: "FPS"
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 1
                        to: 60
                        value: editor.pose && editor.pose.sprite ? editor.pose.sprite.fps : 12
                        editable: true
                        Accessible.name: editor.app.t("Fotogramas por segundo", "Frames per second")
                        onValueModified: editor.editPose(p => p.sprite.fps = value)
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.line
                    Layout.topMargin: 12
                    Layout.bottomMargin: 8
                }
                Text {
                    text: editor.app.t("Cuando habla", "When speaking")
                    color: Theme.text
                    font.weight: Font.DemiBold
                    font.pixelSize: 16
                }
                Text {
                    text: editor.app.t("Brillo", "Brightness")
                    color: Theme.muted
                }
                Slider {
                    Layout.fillWidth: true
                    from: 1
                    to: 2
                    stepSize: 0.01
                    value: editor.character.effects.brightness
                    Accessible.name: editor.app.t("Brillo al hablar", "Speaking brightness")
                    onMoved: editor.app.editCharacter(editor.app.characterId, c => c.effects.brightness = value)
                }
                Text {
                    text: editor.app.t("Salto", "Hop")
                    color: Theme.muted
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: 100
                    value: editor.character.effects.jump
                    Accessible.name: editor.app.t("Salto al hablar", "Speaking hop")
                    onMoved: editor.app.editCharacter(editor.app.characterId, c => c.effects.jump = Math.round(value))
                }
                Check {
                    Layout.fillWidth: true
                    text: editor.app.t("Añadir resplandor", "Add glow")
                    checked: editor.character.effects.glow
                    onToggled: editor.app.editCharacter(editor.app.characterId, c => c.effects.glow = checked)
                }
                GridLayout {
                    columns: 2
                    Layout.fillWidth: true
                    Text {
                        text: editor.app.t("Transición (ms)", "Transition (ms)")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 0
                        to: 1000
                        stepSize: 10
                        value: editor.character.effects.duration_ms
                        editable: true
                        Accessible.name: editor.app.t("Duración de transición", "Transition duration")
                        onValueModified: editor.app.editCharacter(editor.app.characterId, c => c.effects.duration_ms = value)
                    }
                    Text {
                        text: editor.app.t("Reposo tras hablar (ms)", "Release delay (ms)")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 0
                        to: 5000
                        stepSize: 10
                        value: editor.character.effects.release_ms
                        editable: true
                        Accessible.name: editor.app.t("Espera al terminar de hablar", "Speaking release delay")
                        onValueModified: editor.app.editCharacter(editor.app.characterId, c => c.effects.release_ms = value)
                    }
                }
                Text {
                    text: editor.app.t("Brillo en reposo", "Idle brightness")
                    color: Theme.muted
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0.2
                    to: 1
                    stepSize: 0.01
                    value: editor.character.effects.dim_idle
                    Accessible.name: editor.app.t("Brillo en reposo", "Idle brightness")
                    onMoved: editor.app.editCharacter(editor.app.characterId, c => c.effects.dim_idle = value)
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: editor.app.t("Exportar personaje…", "Export character…")
                        onClicked: editor.app.backend.pickFile("export_character", editor.app.characterId)
                    }
                    ActionButton {
                        text: editor.app.t("Eliminar", "Delete")
                        destructive: true
                        quiet: true
                        onClicked: deletion.open()
                    }
                }
            }
        }
        ActionButton {
            text: editor.app.t("Listo", "Done")
            primary: true
            Layout.fillWidth: true
            onClicked: editor.close()
        }
    }
    Dialog {
        id: deletion
        anchors.centerIn: parent
        modal: true
        title: editor.app.t("Eliminar personaje", "Delete character")
        standardButtons: Dialog.Yes | Dialog.Cancel
        Label {
            text: editor.app.t("Se quitará del borrador y de sus invitados. Puedes deshacerlo.", "Removed from the draft and its guests. You can undo this.")
            wrapMode: Text.Wrap
            width: Math.min(editor.width - 70, 300)
        }
        onAccepted: {
            editor.app.edit(d => {
                const id = editor.app.characterId;
                d.characters = d.characters.filter(c => c.id !== id);
                for (const p of Object.values(d.people))
                    if (p.character === id)
                        p.character = null;
                for (const p of d.profiles)
                    for (const g of p.groups)
                        for (const m of g.members)
                            if (m.character === id)
                                m.character = null;
            });
            editor.close();
        }
    }
}
