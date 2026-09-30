pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic

TextField {
    id: control
    property string labelText: placeholderText
    implicitHeight: 44
    implicitWidth: 200
    leftPadding: 12
    rightPadding: 12
    color: Theme.text
    placeholderTextColor: Theme.muted
    selectionColor: "#3b5631"
    selectedTextColor: Theme.text
    font.family: Theme.family
    font.pixelSize: 14
    selectByMouse: true
    Accessible.name: labelText
    background: Rectangle {
        radius: 9
        color: "#171c18"
        border.color: control.activeFocus ? Theme.accent : Theme.line
        border.width: control.activeFocus ? 2 : 1
    }
}
