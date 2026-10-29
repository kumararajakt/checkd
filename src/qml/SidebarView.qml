pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.checkd

Kirigami.ScrollablePage {
    id: sidebarView

    title: ""

    signal openSettingsRequested

    property int _updateTrigger: 0
    property bool _restored: false
    property alias count: navList.count
    property alias currentIndex: navList.currentIndex
    readonly property var currentItem: {
        if (_updateTrigger < 0) {
            return null;
        }
        return (currentIndex >= 0 && currentIndex < navList.count && navList.model) ? navList.model.get(currentIndex) : null;
    }

    function restoreSelection() {
        if (navList.count > 0) {
            const lastId = AppSettings.lastSelectedListId;
            if (lastId && lastId.length > 0 && navList.model && typeof navList.model.indexOfId === "function") {
                const idx = navList.model.indexOfId(lastId);
                if (idx >= 0) {
                    navList.currentIndex = idx;
                }
            }
            _restored = true;
        }
    }

    Connections {
        target: navList.model
        function onDataChanged() {
            sidebarView._updateTrigger++;
        }
        function onModelReset() {
            sidebarView._updateTrigger++;
            sidebarView.restoreSelection();
        }
        function onCountChanged() {
            sidebarView._updateTrigger++;
            sidebarView.restoreSelection();
        }
    }

    function focusNewList() {
        sidebarHeader.showAddList = true;
        addListField.forceActiveFocus();
    }

    // ── Add list ─────────────────────────────────────────────────────────

    header: Kirigami.AbstractApplicationHeader {
        id: sidebarHeader
        property bool showAddList: false

        RowLayout {
            id: headerRow
            spacing: Kirigami.Units.smallSpacing
            width: parent.width

            Item {
                Layout.fillWidth: true
            }

            Kirigami.ActionTextField {
                id: addListField
                visible: sidebarHeader.showAddList
                Layout.fillWidth: true
                placeholderText: qsTr("New List")

                onVisibleChanged: {
                    if (visible) {
                        forceActiveFocus();
                    }
                }

                leftActions: [
                    Kirigami.Action {
                        icon.name: "list-add"
                        onTriggered: addListField.accepted()
                    }
                ]

                rightActions: [
                    Kirigami.Action {
                        icon.name: "dialog-close"
                        tooltip: qsTr("Cancel")
                        onTriggered: {
                            addListField.text = "";
                            sidebarHeader.showAddList = false;
                        }
                    }
                ]

                Keys.onEscapePressed: {
                    addListField.text = "";
                    sidebarHeader.showAddList = false;
                }

                onAccepted: {
                    if (text.trim().length > 0) {
                        sidebarModel.addList(text.trim());
                        text = "";
                        navList.currentIndex = navList.count - 1;
                        sidebarHeader.showAddList = false;
                    }
                }
            }

            QQC2.ToolButton {
                id: addListBtn
                icon.name: "list-add"
                text: qsTr("Add List")
                visible: !sidebarHeader.showAddList
                display: QQC2.AbstractButton.TextBesideIcon
                QQC2.ToolTip.text: text
                QQC2.ToolTip.visible: hovered

                onClicked: sidebarHeader.showAddList = true
            }
        }
    }

    // ── Sidebar List ─────────────────────────────────────────────────────────
    ListView {
        id: navList

        anchors.fill: parent
        currentIndex: -1

        onCurrentIndexChanged: {
            if (currentIndex >= 0 && currentIndex < navList.count && navList.model) {
                const item = navList.model.get(currentIndex);
                if (item && item.id) {
                    AppSettings.lastSelectedListId = item.id;
                }
            } else if (currentIndex === -1 && sidebarView._restored && navList.count === 0) {
                AppSettings.lastSelectedListId = "";
            }
        }

        model: SidebarListModel {
            id: sidebarModel
        }

        delegate: SidebarListDelegate {}
    }

    footer: RowLayout {
        width: parent ? parent.width : 0
        spacing: 0

        QQC2.ItemDelegate {
            Layout.fillWidth: true
            text: qsTr("Settings")
            icon.name: "settings-configure"
            onClicked: sidebarView.openSettingsRequested()
        }

        QQC2.ToolButton {
            visible: SyncManager.available && SyncManager.signedIn
            icon.name: "view-refresh"
            enabled: !SyncManager.syncing
            QQC2.ToolTip.text: SyncManager.syncing ? qsTr("Syncing…") : qsTr("Sync with Google")
            QQC2.ToolTip.visible: hovered
            onClicked: SyncManager.sync()
            RotationAnimator on rotation {
                running: SyncManager.syncing
                from: 0
                to: 360
                loops: Animation.Infinite
                duration: 1000
            }
        }
    }
}
