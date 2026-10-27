pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.Page {
    id: root

    property int selectedListIndex: -1
    property int listsCount: 0
    property var selectedList: null

    signal createListRequested()

    title: root.selectedList ? root.selectedList.label : ""

    // ── 1. When no list is present (count == 0) ─────────────────────────
    Kirigami.PlaceholderMessage {
        id: emptyListPlaceholder
        anchors.centerIn: parent
        width: Math.min(parent.width - Kirigami.Units.largeSpacing * 4, Kirigami.Units.gridUnit * 25)
        visible: root.listsCount === 0

        icon.name: "list-add"
        text: qsTr("Create a new list")
        explanation: qsTr("Get started by adding your first list in the sidebar.")

        helpfulAction: Kirigami.Action {
            icon.name: "list-add"
            text: qsTr("Create a new list")
            onTriggered: root.createListRequested()
        }
    }

    // ── 2. When lists exist but none is selected ────────────────────────
    Kirigami.PlaceholderMessage {
        id: noSelectionPlaceholder
        anchors.centerIn: parent
        width: Math.min(parent.width - Kirigami.Units.largeSpacing * 4, Kirigami.Units.gridUnit * 25)
        visible: root.listsCount > 0 && root.selectedListIndex < 0

        icon.name: "view-list-symbolic"
        text: qsTr("Select a sidebar item")
        explanation: qsTr("Choose a list from the sidebar to view its tasks.")
    }

    // ── 3. Content when a list is selected ──────────────────────────────
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.largeSpacing
        visible: root.listsCount > 0 && root.selectedListIndex >= 0


        Kirigami.PlaceholderMessage {
            Layout.fillWidth: true
            Layout.fillHeight: true
            icon.name: "checkbox"
            text: qsTr("No tasks")
            explanation: qsTr("Tasks in this list will appear here.")
        }
    }
}
