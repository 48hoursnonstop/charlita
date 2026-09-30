pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic

Button {
    id: control
    property bool primary: false
    property bool quiet: false
    property bool destructive: false
    implicitHeight: 42
    implicitWidth: Math.max(42, contentItem.implicitWidth + 30)
    leftPadding: 15
    rightPadding: 15
    hoverEnabled: true
    font.family: Theme.family
    font.pixelSize: 14
    font.weight: Font.Medium
    Accessible.name: text
    contentItem: Text {
        text: control.text
        font: control.font
        color: control.primary ? Theme.accentInk : control.destructive ? Theme.danger : Theme.text
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        opacity: control.enabled ? 1 : 0.5
    }
    background: Rectangle {
        radius: 10
        color: control.primary ? (control.down ? "#98be77" : control.hovered ? "#c4e8a5" : Theme.accent) : control.down ? "#303b33" : control.hovered ? Theme.elevated : control.quiet ? "transparent" : Theme.surface
        border.width: control.activeFocus ? 2 : control.primary || control.quiet ? 0 : 1
        border.color: control.activeFocus ? Theme.accent : Theme.line
        opacity: control.enabled ? 1 : 0.55
    }
}
