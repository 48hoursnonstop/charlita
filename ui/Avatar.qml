pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import QtMultimedia

Item {
    id: avatar
    required property var app
    property var guest: null
    property var pose: guest ? guest.pose : null
    property var media: guest ? guest.media : null
    property string label: guest ? guest.name : ""
    property bool names: false
    property bool interactive: false
    property bool playing: visible && app.visible && app.visibility !== Window.Minimized && !app.document.settings.reduced_motion
    property string stateName: guest ? guest.state : "idle"
    property bool speaking: false
    onStateNameChanged: {
        if (stateName === "speaking") {
            releaseTimer.stop();
            speaking = true;
        } else if (stateName !== "idle") {
            releaseTimer.stop();
            speaking = false;
        } else if (speaking) {
            releaseTimer.interval = guest ? guest.effects.release_ms : 0;
            releaseTimer.restart();
        }
    }
    Timer {
        id: releaseTimer
        onTriggered: avatar.speaking = false
    }
    property real jump: speaking && playing ? guest.effects.jump : 0
    property int spriteFrame: 0
    property var sprite: pose ? pose.sprite : null
    property var crop: pose && pose.crop ? pose.crop : [0, 0, 1, 1]
    property real imageWidth: media ? media.width / (sprite ? sprite.columns : 1) : width
    property real imageHeight: media ? media.height / (sprite ? sprite.rows : 1) : height
    property real fit: Math.min(width / (imageWidth * crop[2]), height / (imageHeight * crop[3]))
    signal activated
    signal nudged(real dx, real dy)
    focus: false
    activeFocusOnTab: interactive
    Accessible.role: interactive ? Accessible.Button : Accessible.Graphic
    Accessible.name: label + (guest ? " · " + guest.state : "")
    Accessible.onPressAction: activated()
    Keys.onReturnPressed: activated()
    Keys.onEnterPressed: activated()
    Keys.onLeftPressed: event => {
        if (interactive) {
            nudged(event.modifiers & Qt.ShiftModifier ? -10 : -1, 0);
            event.accepted = true;
        }
    }
    Keys.onRightPressed: event => {
        if (interactive) {
            nudged(event.modifiers & Qt.ShiftModifier ? 10 : 1, 0);
            event.accepted = true;
        }
    }
    Keys.onUpPressed: event => {
        if (interactive) {
            nudged(0, event.modifiers & Qt.ShiftModifier ? -10 : -1);
            event.accepted = true;
        }
    }
    Keys.onDownPressed: event => {
        if (interactive) {
            nudged(0, event.modifiers & Qt.ShiftModifier ? 10 : 1);
            event.accepted = true;
        }
    }
    TapHandler {
        enabled: avatar.interactive
        onTapped: {
            avatar.forceActiveFocus();
            avatar.activated();
        }
    }
    Timer {
        interval: avatar.sprite ? 1000 / avatar.sprite.fps : 1000
        repeat: true
        running: avatar.playing && !!avatar.sprite
        onTriggered: avatar.spriteFrame = (avatar.spriteFrame + 1) % avatar.sprite.frames
    }
    property string poseSignature: pose ? JSON.stringify(pose) : ""
    onPoseSignatureChanged: spriteFrame = 0
    Item {
        id: art
        width: parent.width
        height: parent.height
        y: -avatar.jump
        visible: !effect.visible
        Behavior on y {
            NumberAnimation {
                duration: avatar.playing && avatar.guest ? avatar.guest.effects.duration_ms : 0
                easing.type: Easing.OutCubic
            }
        }
        Item {
            id: clipping
            anchors.centerIn: parent
            width: avatar.media ? avatar.imageWidth * avatar.crop[2] * avatar.fit : parent.width
            height: avatar.media ? avatar.imageHeight * avatar.crop[3] * avatar.fit : parent.height
            clip: true
            transform: Scale {
                origin.x: clipping.width / 2
                xScale: avatar.guest && avatar.guest.mirror ? -1 : 1
            }
            Loader {
                id: loader
                active: !!avatar.media
                width: avatar.media ? avatar.media.width * avatar.fit : 0
                height: avatar.media ? avatar.media.height * avatar.fit : 0
                x: -((avatar.spriteFrame % (avatar.sprite ? avatar.sprite.columns : 1)) * avatar.imageWidth + avatar.crop[0] * avatar.imageWidth) * avatar.fit
                y: -(Math.floor(avatar.spriteFrame / (avatar.sprite ? avatar.sprite.columns : 1)) * avatar.imageHeight + avatar.crop[1] * avatar.imageHeight) * avatar.fit
                sourceComponent: avatar.media && avatar.media.kind === "webm" ? videoComponent : avatar.media && ["gif", "webp"].indexOf(avatar.media.kind) >= 0 && !avatar.sprite ? animationComponent : imageComponent
            }
            Rectangle {
                anchors.fill: parent
                radius: width / 2
                visible: !avatar.media
                color: "#303d32"
                border.color: "#4d5c4a"
                Text {
                    anchors.centerIn: parent
                    text: avatar.label.slice(0, 2).toUpperCase()
                    color: Theme.accent
                    font.family: Theme.family
                    font.pixelSize: parent.width * 0.25
                    font.weight: Font.Medium
                }
            }
        }
    }
    MultiEffect {
        id: effect
        anchors.fill: art
        source: art
        visible: avatar.speaking || (avatar.guest && avatar.guest.effects.dim_idle !== 1)
        brightness: avatar.guest ? Math.min(1, (avatar.speaking ? avatar.guest.effects.brightness : avatar.guest.effects.dim_idle) - 1) : 0
        shadowEnabled: avatar.speaking && avatar.guest.effects.glow
        shadowColor: "#55ffffff"
        shadowBlur: 0.3
    }
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        radius: 8
        border.width: 2
        border.color: Theme.accent
        visible: avatar.activeFocus && avatar.interactive
    }
    Text {
        anchors.top: parent.bottom
        anchors.topMargin: 8
        anchors.horizontalCenter: parent.horizontalCenter
        width: Math.max(100, parent.width + 40)
        horizontalAlignment: Text.AlignHCenter
        text: avatar.label
        visible: avatar.names
        color: Theme.text
        elide: Text.ElideRight
        font.family: Theme.family
        font.pixelSize: 14
        font.weight: Font.Medium
        ToolTip.visible: nameHover.hovered && truncated
        ToolTip.text: avatar.label
        HoverHandler {
            id: nameHover
        }
    }
    Component {
        id: imageComponent
        Image {
            source: avatar.app.source(avatar.pose, avatar.media)
            asynchronous: true
            cache: true
            smooth: true
            fillMode: Image.Stretch
            sourceSize.width: avatar.media ? Math.min(2048, avatar.media.width, Math.max(1, Math.ceil(loader.width * 2))) : 0
            sourceSize.height: avatar.media ? Math.min(2048, avatar.media.height, Math.max(1, Math.ceil(loader.height * 2))) : 0
        }
    }
    Component {
        id: animationComponent
        AnimatedImage {
            source: avatar.app.source(avatar.pose, avatar.media)
            playing: avatar.playing
            asynchronous: true
            cache: false
            smooth: true
            fillMode: Image.Stretch
            onPlayingChanged: {
                if (!playing)
                    currentFrame = 0;
            }
        }
    }
    Component {
        id: videoComponent
        Item {
            VideoOutput {
                id: video
                anchors.fill: parent
                fillMode: VideoOutput.Stretch
            }
            MediaPlayer {
                id: player
                source: avatar.app.source(avatar.pose, avatar.media)
                videoOutput: video
                loops: MediaPlayer.Infinite
                Component.onCompleted: {
                    if (avatar.playing)
                        play();
                    else {
                        play();
                        pause();
                    }
                }
            }
            Connections {
                target: avatar
                function onPlayingChanged() {
                    if (avatar.playing)
                        player.play();
                    else
                        player.pause();
                }
            }
        }
    }
}
