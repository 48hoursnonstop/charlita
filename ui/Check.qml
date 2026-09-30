pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic

CheckBox {
    id: control
    implicitHeight: Math.max(38, contentItem.implicitHeight + 12)
    font.family: Theme.family
    font.pixelSize: 14
    Accessible.name: text
    contentItem: Text {
        text: control.text
        font: control.font
        color: Theme.text
        wrapMode: Text.WordWrap
        verticalAlignment: Text.AlignVCenter
        leftPadding: control.indicator.width + control.spacing
        opacity: control.enabled ? 1 : 0.5
    }
    background: Rectangle {
        color: "transparent"
        radius: 8
        border.color: Theme.accent
        border.width: control.activeFocus ? 2 : 0
    }
}
