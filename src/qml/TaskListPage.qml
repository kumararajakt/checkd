pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

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
                    dummyTaskModel.append({
                        title: text.trim(),
                        description: "",
                        dueDate: "",
                        dueTime: "",
                        repeatRule: "",
                        repeatConfig: "",
                        isCompleted: false
                    });
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

        model: ListModel {
            id: dummyTaskModel

            ListElement {
                title: "Design user interface mockups"
                description: "Create initial UI wireframes and visual specs for the task list and sidebar."
                dueDate: "2026-10-30"
                dueTime: "10:00"
                repeatRule: "Weekly on Fri"
                repeatConfig: "{\"type\":\"week\",\"interval\":1,\"startDate\":\"2026-10-30\",\"time\":\"10:00\",\"days\":[5]}"
                isCompleted: true
            }
            ListElement {
                title: "Implement sidebar list navigation"
                description: "Add list model, item delegate, and selection handling."
                dueDate: "2026-11-02"
                dueTime: "14:30"
                repeatRule: ""
                repeatConfig: ""
                isCompleted: true
            }
            ListElement {
                title: "Add color picker dialog"
                description: "Design 10-color popup palette with custom native dialog option."
                dueDate: ""
                dueTime: ""
                repeatRule: ""
                repeatConfig: ""
                isCompleted: true
            }
            ListElement {
                title: "Hook up task completion checkboxes"
                description: "Persist checkbox states and apply strikethrough styling."
                dueDate: "2026-10-21"
                dueTime: "08:00"
                repeatRule: "Daily at 08:00"
                repeatConfig: "{\"type\":\"day\",\"interval\":1,\"startDate\":\"2026-10-21\",\"time\":\"08:00\"}"
                isCompleted: false
            }
            ListElement {
                title: "Write unit tests for task models"
                description: "Cover list creation, renaming, color changes, and task completion."
                dueDate: ""
                dueTime: ""
                repeatRule: ""
                repeatConfig: ""
                isCompleted: false
            }
            ListElement {
                title: "Refactor Kirigami application styles"
                description: "Ensure KDE HIG adherence and dark/light Breeze theme support."
                dueDate: "2026-11-10"
                dueTime: "18:00"
                repeatRule: ""
                repeatConfig: ""
                isCompleted: false
            }
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
