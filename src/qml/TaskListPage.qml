pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.checkd

Kirigami.Page {
    id: root

    property int selectedListIndex: -1
    property int listsCount: 0
    property var selectedList: null
    property bool showAddTask: false
    property bool completedExpanded: false

    onSelectedListIndexChanged: {
        root.completedExpanded = false;
    }

    onShowAddTaskChanged: {
        addTaskAction.checked = root.showAddTask;
        if (root.showAddTask) {
            newTaskField.forceActiveFocus();
        }
    }

    signal createListRequested
    signal openSettingsRequested

    title: root.selectedList ? root.selectedList.label : ""

    actions: [
        Kirigami.Action {
            id: addTaskAction
            text: qsTr("Add Task")
            icon.name: "list-add"
            displayHint: Kirigami.DisplayHint.KeepVisible
            checkable: true
            checked: root.showAddTask
            enabled: root.listsCount > 0 && root.selectedListIndex >= 0
            onTriggered: {
                root.showAddTask = checked;
            }
        },
        Kirigami.Action {
            id: settingsAction
            text: qsTr("Settings")
            icon.name: "settings-configure"
            onTriggered: {
                root.openSettingsRequested();
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

    TaskListModel {
        id: taskModel
        listId: root.selectedList ? root.selectedList.id : ""
    }

    TaskFilterProxyModel {
        id: activeTasksModel
        sourceTaskListModel: taskModel
        filterCompleted: false
    }

    TaskFilterProxyModel {
        id: completedTasksModel
        sourceTaskListModel: taskModel
        filterCompleted: true
    }

    // ── 3. Content when a list is selected ──────────────────────────────
    ColumnLayout {
        id: mainContentLayout
        anchors.fill: parent
        spacing: 0
        visible: root.listsCount > 0 && root.selectedListIndex >= 0

        // ── Active Tasks List ───────────────────────────────────────────
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: activeTaskList
                anchors.fill: parent
                clip: true

                model: activeTasksModel
                delegate: TaskItemDelegate {}

                QQC2.BusyIndicator {
                    anchors.centerIn: parent
                    running: taskModel.busy
                    visible: running
                }

                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    visible: taskModel.count === 0 && !taskModel.busy
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

                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    visible: taskModel.count > 0 && activeTasksModel.count === 0 && !taskModel.busy
                    icon.name: "task-complete"
                    text: qsTr("All tasks completed")
                    explanation: qsTr("Completed tasks are shown below.")
                }
            }
        }

        // ── Completed Tasks Section (Bottom) ─────────────────────────────
        ColumnLayout {
            id: completedSection
            Layout.fillWidth: true
            spacing: 0
            visible: completedTasksModel.count > 0

            Kirigami.Separator {
                Layout.fillWidth: true
            }

            QQC2.ItemDelegate {
                id: completedHeader
                Layout.fillWidth: true
                leftPadding: Kirigami.Units.largeSpacing
                rightPadding: Kirigami.Units.largeSpacing
                topPadding: Kirigami.Units.smallSpacing
                bottomPadding: Kirigami.Units.smallSpacing

                contentItem: RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        implicitWidth: Kirigami.Units.iconSizes.small
                        implicitHeight: Kirigami.Units.iconSizes.small
                        source: "go-next"
                        rotation: root.completedExpanded ? 90 : 0
                        opacity: 0.7

                        Behavior on rotation {
                            NumberAnimation {
                                duration: Kirigami.Units.shortDuration
                            }
                        }
                    }

                    QQC2.Label {
                        text: qsTr("Completed (%1)").arg(completedTasksModel.count)
                        font.bold: true
                        Layout.fillWidth: true
                        color: Kirigami.Theme.disabledTextColor
                    }
                }

                onClicked: {
                    root.completedExpanded = !root.completedExpanded;
                }
            }

            ListView {
                id: completedTaskList
                Layout.fillWidth: true
                visible: root.completedExpanded
                clip: true
                Layout.preferredHeight: root.completedExpanded ? Math.min(Kirigami.Units.gridUnit * 12, Math.max(contentHeight, 1)) : 0
                model: completedTasksModel
                delegate: TaskItemDelegate {}

                Behavior on Layout.preferredHeight {
                    NumberAnimation {
                        duration: Kirigami.Units.shortDuration
                    }
                }
            }
        }
    }
}
