pragma Singleton
import QtQuick

QtObject {
    readonly property color background: "#141715"
    readonly property color surface: "#1b201d"
    readonly property color elevated: "#242b26"
    readonly property color line: "#364039"
    readonly property color text: "#edf2ec"
    readonly property color muted: "#a9b5ab"
    readonly property color accent: "#b4db91"
    readonly property color accentInk: "#192319"
    readonly property color danger: "#f1a8a5"
    readonly property string family: Qt.platform.os === "windows" ? "Segoe UI" : "sans-serif"
}
