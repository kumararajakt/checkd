pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.checkd

Kirigami.Page {
    id: root

    property int selectedListIndex: -1
    property int listsCount: 0
    property var selectedList: null
    property bool showAddTask: false

    onShowAddTaskChanged: {
        addTaskAction.checked = root.showAddTask;
        if (root.showAddTask) {
            newTaskField.forceActiveFocus();
        }
    }

    signal createListRequested

    title: root.selectedList ? root.selectedList.label : ""

    actions: [
        Kirigami.Action {
            id: addTaskAction
            text: qsTr("Add Task")
            icon.name: "list-add"
            displayHint: Kirigami.DisplayHint.KeepVisible
            checkable: true
            checked: root.showAddTask
            visible: root.listsCount > 0 && root.selectedListIndex >= 0
            onTriggered: {
                root.showAddTask = checked;
            }
        }
    ]

    header: QQC2.Control {
        id: addTaskHeader
        width: parent ? parent.width : 0
        visible: root.listsCount > 0 && root.selectedListIndex >= 0 && root.showAddTask
        height: visible ? implicitHeight : 0
        leftPadding: Kirigami.Units.largeSpacing
        rightPadding: Kirigami.Units.largeSpacing
        topPadding: Kirigami.Units.mediumSpacing
        bottomPadding: Kirigami.Units.mediumSpacing

        onVisibleChanged: {
            if (visible) {
                newTaskField.forceActiveFocus();
            }
        }

        contentItem: Kirigami.ActionTextField {
            id: newTaskField
            placeholderText: qsTr("  + New Task")
            rightActions: [
                Kirigami.Action {
                    icon.name: "dialog-close"
                    tooltip: qsTr("Close")
                    onTriggered: {
                        newTaskField.text = "";
                        root.showAddTask = false;
                    }
                }
            ]
            Keys.onEscapePressed: {
                newTaskField.text = "";
                root.showAddTask = false;
            }
            onAccepted: {
                if (text.trim().length > 0) {
                    taskModel.addTask(text.trim());
                    text = "";
                }
            }
        }
    }

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
    ListView {
        id: taskList
        anchors.fill: parent

        visible: root.listsCount > 0 && root.selectedListIndex >= 0

        model: TaskListModel {
            id: taskModel
            listId: root.selectedList ? root.selectedList.id : ""
        }

        delegate: TaskItemDelegate {}

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            visible: taskList.count === 0
            icon.name: "checkbox"
            text: qsTr("No tasks")
            explanation: qsTr("No Tasks Found.")

            helpfulAction: Kirigami.Action {
                icon.name: "list-add"
                text: qsTr("Add Task")
                onTriggered: {
                    root.showAddTask = true;
                }
            }
        }
    }
}
