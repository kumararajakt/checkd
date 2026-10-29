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
    property alias count: navList.count
    property alias currentIndex: navList.currentIndex
    readonly property var currentItem: {
        if (_updateTrigger < 0) {
            return null;
        }
        return (currentIndex >= 0 && currentIndex < navList.count && navList.model)
            ? navList.model.get(currentIndex)
            : null;
    }

    Connections {
        target: navList.model
        function onDataChanged() {
            sidebarView._updateTrigger++;
        }
        function onModelReset() {
            sidebarView._updateTrigger++;
        }
        function onCountChanged() {
            sidebarView._updateTrigger++;
        }
    }

    function focusNewList() {
        addListField.forceActiveFocus();
    }

    // ── Add list ─────────────────────────────────────────────────────────

    header: Kirigami.AbstractApplicationHeader {

        QQC2.ItemDelegate {
            width: parent.width

            contentItem: Kirigami.ActionTextField {
                id: addListField
                placeholderText: qsTr("New List")
                Layout.fillWidth: true

                leftActions: [
                    Kirigami.Action {
                        icon.name: "list-add"
                        onTriggered: addListField.accepted()
                    }
                ]

                onAccepted: {
                    if (text.trim().length > 0) {
                        sidebarModel.addList(text.trim());
                        text = "";
                        navList.currentIndex = navList.count - 1;
                    }
                }
            }
        }
    }

    // ── Sidebar List ─────────────────────────────────────────────────────────
    ListView {
        id: navList

        anchors.fill: parent
        currentIndex: -1

        model: SidebarListModel {
            id: sidebarModel
        }

        delegate: SidebarListDelegate {}
    }

    footer: QQC2.ItemDelegate {
        width: parent ? parent.width : 0
        text: qsTr("Settings")
        icon.name: "settings-configure"
        onClicked: sidebarView.openSettingsRequested()
    }
}
