pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

Item {
    id: scene
    objectName: "scenePage"
    property bool expanded: false
    required property var app
    property var backend: app.backend
    ListModel {
        id: actors
        dynamicRoles: true
    }
    function sync() {
        const guests = app.output.guests;
        for (let i = 0; i < guests.length; ++i) {
            let index = -1;
            for (let j = i; j < actors.count; ++j)
                if (actors.get(j).guest.id === guests[i].id) {
                    index = j;
                    break;
                }
            if (index < 0)
                actors.insert(i, {
                    guest: guests[i]
                });
            else {
                if (index !== i)
                    actors.move(index, i, 1);
                actors.set(i, {
                    guest: guests[i]
                });
            }
        }
        if (actors.count > guests.length)
            actors.remove(guests.length, actors.count - guests.length);
    }
    Component.onCompleted: sync()
    Connections {
        target: scene.app
        function onOutputChanged() {
            scene.sync();
        }
    }
    FocusScroll {
        anchors.fill: parent
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
            width: scene.width
            height: Math.max(implicitHeight, scene.height)
            spacing: 18
            GridLayout {
                Layout.fillWidth: true
                columns: scene.width < 600 ? 1 : 2
                rowSpacing: 12
                columnSpacing: 12
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ComboBox {
                        width: 165
                        height: 40
                        font.pixelSize: 13
                        model: scene.app.document.profiles
                        textRole: "name"
                        valueRole: "id"
                        currentIndex: scene.app.document.profiles.findIndex(p => p.id === scene.app.profileId)
                        onActivated: {
                            scene.app.profileId = currentValue;
                            scene.app.groupId = scene.app.profile.groups[0].id;
                        }
                        Accessible.name: scene.app.t("Perfil", "Profile")
                    }
                    Repeater {
                        model: scene.app.profile.groups
                        ActionButton {
                            required property var modelData
                            text: modelData.name
                            primary: scene.app.group.id === modelData.id
                            quiet: scene.app.group.id !== modelData.id
                            onClicked: scene.app.groupId = modelData.id
                        }
                    }
                    ActionButton {
                        text: "+"
                        Accessible.name: scene.app.t("Crear grupo", "Create group")
                        quiet: true
                        onClicked: {
                            createDialog.kind = "group";
                            createDialog.open();
                        }
                    }
                    ActionButton {
                        text: scene.app.t("Perfiles", "Profiles")
                        quiet: true
                        onClicked: profilesDialog.open()
                    }
                }
                ColumnLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 4
                    ActionButton {
                        objectName: "applyChanges"
                        text: scene.app.t("Aplicar cambios", "Apply changes")
                        primary: true
                        enabled: !scene.app.busy
                        onClicked: scene.backend.request("apply")
                    }
                    Text {
                        text: scene.backend.state.dirty ? scene.app.t("Borrador guardado", "Draft saved") : scene.app.t("Todo aplicado", "All applied")
                        color: Theme.muted
                        font.pixelSize: 12
                        Layout.alignment: Qt.AlignRight
                    }
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: stageHeader.implicitHeight + outputActions.implicitHeight + (actors.count === 0 ? 268 : 176)
                color: Theme.surface
                radius: 18
                border.color: Theme.line
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: scene.width < 650 ? 14 : 22
                    spacing: 16
                    Flow {
                        id: stageHeader
                        Layout.fillWidth: true
                        spacing: 10
                        Text {
                            text: scene.app.group.name
                            color: Theme.text
                            font.pixelSize: 15
                            font.weight: Font.Medium
                            width: Math.min(implicitWidth, stageHeader.width)
                            height: 34
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                            ToolTip.visible: stageNameHover.hovered && truncated
                            ToolTip.text: text
                            HoverHandler {
                                id: stageNameHover
                            }
                        }
                        Text {
                            text: scene.app.output.width + " × " + scene.app.output.height
                            color: Theme.muted
                            font.pixelSize: 12
                            height: 34
                            verticalAlignment: Text.AlignVCenter
                        }
                        ActionButton {
                            text: scene.app.t("Composición", "Composition")
                            quiet: true
                            implicitHeight: 34
                            onClicked: composition.open()
                        }
                        ActionButton {
                            text: scene.expanded ? scene.app.t("Volver", "Back") : scene.app.t("Ampliar", "Expand")
                            quiet: true
                            implicitHeight: 34
                            onClicked: scene.expanded = !scene.expanded
                        }
                    }
                    Item {
                        id: canvas
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.minimumHeight: actors.count === 0 ? 190 : 100
                        property real factor: Math.min(width / scene.app.output.width, height / scene.app.output.height)
                        Rectangle {
                            id: stage
                            anchors.centerIn: parent
                            width: scene.app.output.width * canvas.factor
                            height: scene.app.output.height * canvas.factor
                            color: "#151a16"
                            radius: 8
                            Canvas {
                                anchors.fill: parent
                                onWidthChanged: requestPaint()
                                onHeightChanged: requestPaint()
                                onPaint: {
                                    const c = getContext("2d");
                                    c.clearRect(0, 0, width, height);
                                    c.fillStyle = "#242c25";
                                    for (let y = 18; y < height; y += 24)
                                        for (let x = 18; x < width; x += 24) {
                                            c.beginPath();
                                            c.arc(x, y, 1, 0, Math.PI * 2);
                                            c.fill();
                                        }
                                }
                            }
                            Item {
                                width: scene.app.output.width
                                height: scene.app.output.height
                                scale: canvas.factor
                                transformOrigin: Item.TopLeft
                                Repeater {
                                    model: actors
                                    Avatar {
                                        id: actor
                                        required property var model
                                        app: scene.app
                                        guest: model.guest
                                        x: guest.x
                                        y: guest.y
                                        width: guest.size
                                        height: guest.size
                                        visible: guest.visible
                                        names: scene.app.output.labels
                                        interactive: true
                                        onActivated: scene.app.showGuest(guest.id)
                                        onNudged: (dx, dy) => {
                                            if (scene.app.group.layout === "free")
                                                scene.app.editMember(guest.id, m => {
                                                    if (!m.locked) {
                                                        m.x += dx;
                                                        m.y += dy;
                                                    }
                                                });
                                        }
                                        DragHandler {
                                            enabled: scene.app.group.layout === "free" && !scene.app.group.members.find(m => m.user === actor.guest.id)?.locked
                                            onActiveChanged: if (!active)
                                                scene.app.editMember(actor.guest.id, m => {
                                                    m.x = actor.x;
                                                    m.y = actor.y;
                                                })
                                        }
                                    }
                                }
                            }
                            ColumnLayout {
                                parent: canvas
                                anchors.centerIn: parent
                                width: Math.min(parent.width - 32, 350)
                                spacing: 10
                                visible: actors.count === 0
                                Text {
                                    text: scene.app.t("La escena está lista para tu gente", "The stage is ready for your people")
                                    Layout.fillWidth: true
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.Wrap
                                    color: Theme.text
                                    font.pixelSize: 18
                                    font.weight: Font.Medium
                                }
                                Text {
                                    text: scene.app.t("Añade un invitado o conecta Discord para encontrar a quienes están en la llamada.", "Add a guest or connect Discord to find people in your call.")
                                    Layout.fillWidth: true
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.Wrap
                                    color: Theme.muted
                                    font.pixelSize: 13
                                }
                                ActionButton {
                                    text: scene.app.t("Añadir invitado", "Add guest")
                                    Layout.alignment: Qt.AlignHCenter
                                    onClicked: addGuest.open()
                                }
                            }
                        }
                    }
                    Flow {
                        id: outputActions
                        Layout.fillWidth: true
                        spacing: 8
                        ActionButton {
                            text: scene.app.t("Copiar URL para OBS / Streamlabs", "Copy URL for OBS / Streamlabs")
                            onClicked: scene.app.copyUrl("group", scene.app.group.id)
                        }
                        ActionButton {
                            text: scene.app.t("Ver salida", "View output")
                            quiet: true
                            onClicked: scene.backend.open(scene.backend.state.baseUrl + "/group/" + scene.app.group.id)
                        }
                        ActionButton {
                            text: scene.app.t("Deshacer", "Undo")
                            quiet: true
                            enabled: scene.backend.state.undo
                            onClicked: scene.backend.request("undo")
                        }
                        ActionButton {
                            text: scene.app.t("Rehacer", "Redo")
                            quiet: true
                            enabled: scene.backend.state.redo
                            onClicked: scene.backend.request("redo")
                        }
                    }
                }
            }
            RowLayout {
                visible: !scene.expanded
                Layout.fillWidth: true
                Text {
                    text: scene.app.t("Tu gente", "Your people")
                    color: Theme.text
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                    Layout.fillWidth: true
                }
                ActionButton {
                    text: scene.app.t("+ Añadir", "+ Add")
                    quiet: true
                    implicitHeight: 34
                    onClicked: addGuest.open()
                }
                ActionButton {
                    text: "···"
                    Accessible.name: scene.app.t("Importar y exportar proyecto", "Import and export project")
                    quiet: true
                    implicitHeight: 34
                    onClicked: projectMenu.open()
                }
                Menu {
                    id: projectMenu
                    MenuItem {
                        text: scene.app.t("Importar proyecto…", "Import project…")
                        onTriggered: scene.backend.pickFile("import_project")
                    }
                    MenuItem {
                        text: scene.app.t("Exportar proyecto…", "Export project…")
                        onTriggered: scene.backend.pickFile("export_project")
                    }
                }
            }
            FocusScroll {
                id: guestScroll
                visible: !scene.expanded
                Layout.fillWidth: true
                Layout.preferredHeight: 92
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AsNeeded
                ScrollBar.vertical.policy: ScrollBar.AlwaysOff
                Row {
                    spacing: 10
                    Repeater {
                        model: scene.app.group.members
                        Rectangle {
                            id: personCard
                            required property var modelData
                            property var person: scene.app.document.people[modelData.user]
                            property var presence: scene.backend.state.runtime.users[modelData.user] || ({})
                            width: 190
                            height: 82
                            radius: 12
                            color: Theme.surface
                            border.color: presence.speaking ? Theme.accent : Theme.line
                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 12
                                spacing: 10
                                Avatar {
                                    Layout.preferredWidth: 40
                                    Layout.preferredHeight: 40
                                    app: scene.app
                                    guest: scene.app.output.guests.find(g => g.id === personCard.modelData.user) || null
                                    label: personCard.person.name
                                    playing: false
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 5
                                    Text {
                                        text: personCard.person.name
                                        color: Theme.text
                                        font.pixelSize: 13
                                        font.weight: Font.Medium
                                        Layout.fillWidth: true
                                        elide: Text.ElideRight
                                        ToolTip.visible: cardHover.hovered && truncated
                                        ToolTip.text: text
                                    }
                                    Text {
                                        text: personCard.presence.hidden ? scene.app.t("Oculto", "Hidden") : personCard.presence.speaking ? scene.app.t("Hablando", "Speaking") : scene.app.t("En reposo", "At rest")
                                        color: Theme.muted
                                        font.pixelSize: 12
                                    }
                                }
                            }
                            HoverHandler {
                                id: cardHover
                            }
                            Button {
                                anchors.fill: parent
                                text: personCard.person.name
                                Accessible.name: scene.app.t("Editar invitado ", "Edit guest ") + text
                                background: Rectangle {
                                    color: "transparent"
                                    radius: 12
                                    border.width: parent.activeFocus ? 2 : 0
                                    border.color: Theme.accent
                                }
                                contentItem: Item {}
                                onActiveFocusChanged: if (activeFocus) {
                                    const flick = guestScroll.contentItem as Flickable;
                                    const left = personCard.x;
                                    if (left < flick.contentX)
                                        flick.contentX = left;
                                    else if (left + personCard.width > flick.contentX + flick.width)
                                        flick.contentX = left + personCard.width - flick.width;
                                }
                                onClicked: scene.app.showGuest(personCard.modelData.user)
                            }
                        }
                    }
                }
            }
        }
    }
    Dialog {
        id: createDialog
        property string kind: "group"
        anchors.centerIn: parent
        title: kind === "group" ? scene.app.t("Nuevo grupo", "New group") : scene.app.t("Nuevo perfil", "New profile")
        modal: true
        width: Math.min(scene.width, 400)
        standardButtons: Dialog.Ok | Dialog.Cancel
        Field {
            id: newName
            width: parent.width
            placeholderText: scene.app.t("Nombre", "Name")
            text: createDialog.kind === "group" ? scene.app.t("Nuevo grupo", "New group") : scene.app.t("Nuevo perfil", "New profile")
        }
        onAccepted: {
            const r = scene.backend.request("create", {
                kind: kind,
                name: newName.text,
                profile: scene.app.profileId
            });
            if (r.ok) {
                if (kind === "group")
                    scene.app.groupId = r.value.id;
                else
                    scene.app.profileId = r.value.id;
            }
        }
    }
    Dialog {
        id: profilesDialog
        anchors.centerIn: parent
        modal: true
        width: Math.min(scene.width, 430)
        title: scene.app.t("Perfiles", "Profiles")
        standardButtons: Dialog.Close
        ColumnLayout {
            width: parent.width
            Text {
                text: scene.app.t("Nombre del perfil", "Profile name")
                color: Theme.muted
            }
            Field {
                Layout.fillWidth: true
                text: scene.app.profile.name || ""
                labelText: scene.app.t("Nombre del perfil", "Profile name")
                onEditingFinished: scene.app.edit(d => {
                    d.profiles.find(p => p.id === scene.app.profileId).name = text;
                })
            }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                ActionButton {
                    text: scene.app.t("Crear perfil", "Create profile")
                    onClicked: {
                        profilesDialog.close();
                        createDialog.kind = "profile";
                        createDialog.open();
                    }
                }
                ActionButton {
                    text: scene.app.t("Duplicar", "Duplicate")
                    onClicked: {
                        const r = scene.backend.request("duplicate", {
                            kind: "profile",
                            id: scene.app.profileId
                        });
                        if (r.ok)
                            scene.app.profileId = r.value.id;
                    }
                }
                ActionButton {
                    text: scene.app.t("Eliminar", "Delete")
                    destructive: true
                    enabled: scene.app.document.profiles.length > 1
                    onClicked: {
                        deletion.kind = "profile";
                        deletion.open();
                    }
                }
            }
        }
    }
    Dialog {
        id: addGuest
        anchors.centerIn: parent
        modal: true
        width: Math.min(scene.width, 430)
        title: scene.app.t("Añadir invitado", "Add guest")
        standardButtons: Dialog.Close
        ColumnLayout {
            width: parent.width
            spacing: 12
            Text {
                text: scene.app.t("En la llamada", "In the call")
                color: Theme.text
                font.weight: Font.Medium
            }
            Repeater {
                model: Object.values(scene.backend.state.runtime.discovered || ({})).filter(p => !scene.app.group.members.some(m => m.user === p.id))
                ActionButton {
                    required property var modelData
                    text: "+ " + modelData.name
                    Layout.fillWidth: true
                    onClicked: scene.backend.request("add_guest", {
                        group: scene.app.group.id,
                        user: modelData.id
                    })
                }
            }
            Text {
                text: scene.app.t("También puedes añadir a alguien por su ID de Discord.", "You can also add someone by their Discord ID.")
                color: Theme.muted
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            Field {
                id: guestId
                Layout.fillWidth: true
                placeholderText: scene.app.t("ID de usuario de Discord", "Discord user ID")
                inputMethodHints: Qt.ImhDigitsOnly
            }
            Field {
                id: guestName
                Layout.fillWidth: true
                placeholderText: scene.app.t("Nombre visible", "Display name")
            }
            ActionButton {
                text: scene.app.t("Añadir al borrador", "Add to draft")
                primary: true
                onClicked: {
                    const r = scene.backend.request("add_guest", {
                        group: scene.app.group.id,
                        user: guestId.text,
                        name: guestName.text || "Guest"
                    });
                    if (r.ok) {
                        addGuest.close();
                        scene.app.showGuest(guestId.text);
                    }
                }
            }
        }
    }
    Dialog {
        id: composition
        anchors.centerIn: parent
        modal: true
        width: Math.min(scene.width, 500)
        height: Math.min(scene.height, 570)
        title: scene.app.t("Composición del grupo", "Group composition")
        standardButtons: Dialog.Close
        FocusScroll {
            anchors.fill: parent
            clip: true
            ColumnLayout {
                width: parent.width
                spacing: 12
                Text {
                    text: scene.app.t("Nombre", "Name")
                    color: Theme.muted
                }
                Field {
                    Layout.fillWidth: true
                    text: scene.app.group.name
                    labelText: scene.app.t("Nombre del grupo", "Group name")
                    onEditingFinished: scene.app.editGroup(g => g.name = text)
                }
                Text {
                    text: scene.app.t("Distribución", "Layout")
                    color: Theme.muted
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    Repeater {
                        model: [
                            {
                                id: "row",
                                es: "Horizontal",
                                en: "Horizontal"
                            },
                            {
                                id: "column",
                                es: "Vertical",
                                en: "Vertical"
                            },
                            {
                                id: "grid",
                                es: "Cuadrícula",
                                en: "Grid"
                            },
                            {
                                id: "free",
                                es: "Libre",
                                en: "Free"
                            }
                        ]
                        ActionButton {
                            required property var modelData
                            text: scene.app.es ? modelData.es : modelData.en
                            primary: scene.app.group.layout === modelData.id
                            onClicked: scene.app.editGroup(g => g.layout = modelData.id)
                        }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: scene.app.t("Ancho", "Width")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 64
                        to: 8192
                        value: scene.app.group.width
                        editable: true
                        Accessible.name: scene.app.t("Ancho del grupo", "Group width")
                        onValueModified: scene.app.editGroup(g => g.width = value)
                    }
                    Text {
                        text: scene.app.t("Alto", "Height")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 64
                        to: 8192
                        value: scene.app.group.height
                        editable: true
                        Accessible.name: scene.app.t("Alto del grupo", "Group height")
                        onValueModified: scene.app.editGroup(g => g.height = value)
                    }
                }
                Text {
                    text: scene.app.t("Separación", "Spacing")
                    color: Theme.muted
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: 200
                    value: scene.app.group.gap
                    Accessible.name: scene.app.t("Separación entre invitados", "Guest spacing")
                    onMoved: scene.app.editGroup(g => g.gap = Math.round(value))
                }
                RowLayout {
                    visible: scene.app.group.layout === "grid"
                    Text {
                        text: scene.app.t("Columnas", "Columns")
                        color: Theme.muted
                    }
                    SpinBox {
                        from: 1
                        to: 256
                        value: scene.app.group.columns
                        editable: true
                        Accessible.name: scene.app.t("Columnas", "Columns")
                        onValueModified: scene.app.editGroup(g => g.columns = value)
                    }
                }
                Check {
                    Layout.fillWidth: true
                    text: scene.app.t("Mostrar nombres", "Show names")
                    checked: scene.app.group.labels
                    onToggled: scene.app.editGroup(g => g.labels = checked)
                }
                Check {
                    Layout.fillWidth: true
                    text: scene.app.t("Conservar espacios al ocultar", "Preserve spaces when hidden")
                    checked: scene.app.group.preserve_spaces
                    onToggled: scene.app.editGroup(g => g.preserve_spaces = checked)
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 8
                    ActionButton {
                        text: scene.app.t("Duplicar grupo", "Duplicate group")
                        onClicked: {
                            const r = scene.backend.request("duplicate", {
                                kind: "group",
                                id: scene.app.group.id
                            });
                            if (r.ok)
                                scene.app.groupId = r.value.id;
                        }
                    }
                    ActionButton {
                        text: scene.app.t("Eliminar grupo", "Delete group")
                        destructive: true
                        enabled: scene.app.profile.groups.length > 1
                        onClicked: {
                            deletion.kind = "group";
                            deletion.open();
                        }
                    }
                }
                Text {
                    text: scene.app.t("Se guarda en el borrador. Pulsa Aplicar cambios para actualizar el directo.", "Saved to draft. Select Apply changes to update the stream.")
                    color: Theme.muted
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    font.pixelSize: 12
                }
            }
        }
    }
    Dialog {
        id: deletion
        property string kind: "group"
        anchors.centerIn: parent
        modal: true
        title: scene.app.t("Confirmar eliminación", "Confirm deletion")
        standardButtons: Dialog.Yes | Dialog.Cancel
        Label {
            text: scene.app.t("Se quitará del borrador. Puedes deshacerlo.", "It will be removed from the draft. You can undo this.")
        }
        onAccepted: {
            scene.app.edit(d => {
                if (kind === "profile")
                    d.profiles = d.profiles.filter(p => p.id !== scene.app.profileId);
                else
                    d.profiles.find(p => p.id === scene.app.profileId).groups = scene.app.profile.groups.filter(g => g.id !== scene.app.groupId);
            });
            composition.close();
            profilesDialog.close();
        }
    }
}
