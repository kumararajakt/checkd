pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.settings as KirigamiSettings
import org.checkd

Kirigami.ApplicationWindow {
    id: root

    width: Kirigami.Settings.isMobile ? -1 : Kirigami.Units.gridUnit * 65
    height: Kirigami.Settings.isMobile ? -1 : Kirigami.Units.gridUnit * 45

    title: qsTr("CheckD")

    Component {
        id: generalSettingsComponent
        GeneralSettingsPage {}
    }

    KirigamiSettings.ConfigurationView {
        id: configView
        window: root

        modules: [
            KirigamiSettings.ConfigurationModule {
                moduleId: "general"
                text: qsTr("General")
                icon.name: "settings-configure"
                page: () => generalSettingsComponent
            }
        ]
    }

    Connections {
        target: systemTray
        function onOpenSettingsRequested() {
            root.show();
            root.raise();
            root.requestActivate();
            configView.open();
        }
    }

    globalDrawer: Kirigami.GlobalDrawer {
        id: sidebarDrawer
        modal: !root.wideScreen
        drawerOpen: root.wideScreen
        handleVisible: !root.wideScreen
        width: Kirigami.Units.gridUnit * 14

        contentItem: SidebarView {
            id: sidebar
            onOpenSettingsRequested: {
                configView.open();
            }
        }
    }

    pageStack.initialPage: TaskListPage {
        selectedListIndex: sidebar.currentIndex
        listsCount: sidebar.count
        selectedList: sidebar.currentItem

        onCreateListRequested: {
            if (!sidebarDrawer.drawerOpen) {
                sidebarDrawer.open();
            }
            sidebar.focusNewList();
        }

        onOpenSettingsRequested: {
            configView.open();
        }
    }
}
