pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts

ApplicationWindow {
    id: root
    objectName: "charlitaWindow"
    required property Bridge backend
    width: 1220
    height: 820
    minimumWidth: 320
    minimumHeight: 520
    visible: true
    title: "Charlita"
    color: Theme.background
    font.family: Theme.family
    font.pixelSize: 14
    palette.window: Theme.surface
    palette.base: Theme.background
    palette.text: Theme.text
    palette.windowText: Theme.text
    palette.buttonText: Theme.text
    palette.button: Theme.elevated
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.accentInk
    property var document: backend.state.document || ({
            profiles: [],
            people: {},
            characters: [],
            assets: {},
            settings: {
                language: "es"
            }
        })
    property bool es: document.settings.language === "es"
    property string profileId: ""
    property string groupId: ""
    property string guestId: ""
    property string characterId: ""
    property int page: 0
    property string toast: ""
    property var profile: document.profiles.find(p => p.id === profileId) || document.profiles[0] || ({
            groups: []
        })
    property var group: profile.groups.find(g => g.id === groupId) || profile.groups[0] || ({
            members: [],
            id: "",
            name: ""
        })
    property var output: backend.state.outputs ? backend.state.outputs[group.id] || ({
            guests: [],
            width: 1280,
            height: 720
        }) : ({
            guests: [],
            width: 1280,
            height: 720
        })
    readonly property bool busy: backend.state.busy || false
    function t(spanish, english) {
        return es ? spanish : english;
    }
    function edit(change) {
        if (busy)
            return {
                ok: false
            };
        const copy = JSON.parse(JSON.stringify(document));
        change(copy);
        return backend.request("draft", {
            document: copy
        });
    }
    function editGroup(change) {
        return edit(d => {
            for (const p of d.profiles)
                for (const g of p.groups)
                    if (g.id === group.id)
                        change(g);
        });
    }
    function editMember(user, change) {
        return editGroup(g => {
            const m = g.members.find(m => m.user === user);
            if (m)
                change(m);
        });
    }
    function editPerson(user, change) {
        return edit(d => {
            if (d.people[user])
                change(d.people[user]);
        });
    }
    function editCharacter(id, change) {
        return edit(d => {
            const c = d.characters.find(c => c.id === id);
            if (c)
                change(c);
        });
    }
    function copyUrl(kind, id) {
        backend.copy(backend.state.baseUrl + "/" + kind + "/" + id);
        toast = t("URL copiada", "URL copied");
        toastTimer.restart();
    }
    function source(pose, media) {
        if (pose && document.assets[pose.asset]) {
            const asset = document.assets[pose.asset];
            if (asset.extension === "webm") {
                return backend.state.previews[asset.id] ? backend.localFile(backend.state.assetRoot + "/" + asset.id + ".preview.webp") : "";
            }
            return backend.localFile(backend.state.assetRoot + "/" + asset.id + "." + asset.extension);
        }
        return media ? media.url : "";
    }
    function showGuest(id) {
        guestId = id;
        guestEditor.open();
    }
    function showCharacter(id) {
        characterId = id;
        characterEditor.open();
    }
    onDocumentChanged: {
        if (!document.profiles.some(p => p.id === profileId))
            profileId = document.profiles[0] ? document.profiles[0].id : "";
        const current = document.profiles.find(p => p.id === profileId);
        if (current && !current.groups.some(g => g.id === groupId))
            groupId = current.groups[0].id;
    }
    onClosing: close => {
        if (backend.hasTray) {
            close.accepted = false;
            hide();
        }
    }
    Timer {
        id: toastTimer
        interval: 2400
        onTriggered: root.toast = ""
    }
    Shortcut {
        sequences: [StandardKey.Undo]
        enabled: !root.busy
        onActivated: backend.request("undo")
    }
    Shortcut {
        sequences: [StandardKey.Redo]
        enabled: !root.busy
        onActivated: backend.request("redo")
    }
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: width < 650 ? 16 : 30
        spacing: 20
        RowLayout {
            Layout.fillWidth: true
            spacing: root.width < 560 ? 4 : 10
            Image {
                source: "logo.svg"
                visible: root.width >= 420
                sourceSize.width: 36
                sourceSize.height: 36
                Layout.preferredWidth: 36
                Layout.preferredHeight: 36
            }
            Text {
                text: "charlita"
                color: Theme.text
                font.family: Theme.family
                font.pixelSize: 24
                font.weight: Font.DemiBold
                visible: root.width >= 560
            }
            Item {
                Layout.fillWidth: true
            }
            Repeater {
                model: [root.t("Escena", "Scene"), root.t("Biblioteca", "Library"), root.t("Ajustes", "Settings")]
                ActionButton {
                    required property int index
                    required property string modelData
                    text: modelData
                    implicitWidth: contentItem.implicitWidth + (root.width < 560 ? 18 : 30)
                    font.pixelSize: root.width < 560 ? 13 : 14
                    primary: root.page === index
                    quiet: root.page !== index
                    onClicked: root.page = index
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.line
            opacity: 0.5
        }
        StackLayout {
            enabled: !root.busy
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: root.page
            Scene {
                app: root
            }
            Library {
                app: root
            }
            Settings {
                app: root
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Rectangle {
                implicitWidth: 7
                implicitHeight: 7
                radius: 4
                color: backend.state.runtime && backend.state.runtime.connection === "connected" ? Theme.accent : Theme.muted
            }
            Text {
                Layout.fillWidth: true
                elide: Text.ElideRight
                color: Theme.muted
                font.pixelSize: 12
                text: {
                    const r = backend.state.runtime || {};
                    if (r.connection === "connected")
                        return "Discord · " + (r.channel_name || root.t("conectado", "connected"));
                    if (r.connection === "not_configured")
                        return root.t("Discord sin conectar", "Discord not connected");
                    if (r.connection === "reconnecting")
                        return root.t("Reconectando · personajes en reposo", "Reconnecting · characters at rest");
                    if (r.connection === "authorizing")
                        return root.t("Completa la autorización en Discord", "Complete authorization in Discord");
                    if (r.connection === "authorization_required" || r.connection === "signed_out")
                        return root.t("Autoriza Discord en Ajustes", "Authorize Discord in Settings");
                    return root.t("Conectando con Discord…", "Connecting to Discord…");
                }
            }
            Text {
                text: "v" + (backend.state.version || "0.1.0")
                color: Theme.muted
                font.pixelSize: 12
            }
            ActionButton {
                text: root.t("Salir", "Quit")
                quiet: true
                implicitHeight: 28
                onClicked: backend.quit()
            }
        }
    }
    Rectangle {
        id: noticeBanner
        objectName: "noticeBanner"
        parent: Overlay.overlay
        z: 100
        anchors.bottom: parent.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottomMargin: 56
        width: Math.min(parent.width - 32, notice.implicitWidth + 80)
        height: notice.implicitHeight + 28
        radius: 12
        color: Theme.elevated
        border.color: backend.error || backend.state.error ? Theme.danger : Theme.line
        visible: !!(backend.error || backend.state.error || root.toast || root.busy)
        RowLayout {
            anchors.fill: parent
            anchors.margins: 12
            BusyIndicator {
                visible: root.busy
                running: visible
                implicitWidth: 24
                implicitHeight: 24
            }
            Text {
                id: notice
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.text
                font.pixelSize: 13
                text: backend.error || backend.state.error || (root.busy ? root.t("Procesando archivo…", "Processing file…") : root.toast)
            }
            ActionButton {
                text: "×"
                Accessible.name: root.t("Cerrar aviso", "Dismiss notice")
                quiet: true
                implicitWidth: 28
                implicitHeight: 28
                visible: !root.busy
                onClicked: {
                    backend.dismiss();
                    root.toast = "";
                }
            }
        }
    }
    Shortcut {
        sequence: "Escape"
        enabled: noticeBanner.visible && !root.busy
        onActivated: {
            backend.dismiss();
            root.toast = "";
        }
    }
    GuestEditor {
        id: guestEditor
        app: root
    }
    CharacterEditor {
        id: characterEditor
        app: root
    }
}
