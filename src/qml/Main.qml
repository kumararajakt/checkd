pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root

    width: Kirigami.Settings.isMobile ? -1 : Kirigami.Units.gridUnit * 65
    height: Kirigami.Settings.isMobile ? -1 : Kirigami.Units.gridUnit * 45

    title: qsTr("CheckD")

    globalDrawer: Kirigami.GlobalDrawer {
        id: sidebarDrawer
        modal: !root.wideScreen
        drawerOpen: root.wideScreen
        handleVisible: !root.wideScreen
        width: Kirigami.Units.gridUnit * 14

        contentItem: SidebarView {}
    }

    pageStack.initialPage: Kirigami.Page {
        Controls.Label {
            anchors.centerIn: parent
            text: qsTr("Hello World!")
        }
    }
}
