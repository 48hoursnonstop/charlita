pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic

ScrollView {
    id: view
    clip: true
    ScrollBar.vertical: ScrollBar {
        policy: ScrollBar.AsNeeded
        active: true
    }
    function revealFocus() {
        const window = view.Window.window;
        const item = window ? window.activeFocusItem : null;
        const flick = view.contentItem as Flickable;
        if (!item || !flick)
            return;
        let ancestor = item;
        while (ancestor && ancestor !== view)
            ancestor = ancestor.parent;
        if (!ancestor)
            return;
        const point = item.mapToItem(flick.contentItem, 0, 0);
        let next = flick.contentY;
        if (point.y < next)
            next = point.y - 8;
        else if (point.y + item.height > next + flick.height)
            next = point.y + item.height - flick.height + 8;
        flick.contentY = Math.max(0, Math.min(next, flick.contentHeight - flick.height));
    }
    Connections {
        target: view.Window.window
        function onActiveFocusItemChanged() {
            Qt.callLater(view.revealFocus);
        }
    }
}
