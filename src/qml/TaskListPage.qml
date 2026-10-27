pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

Kirigami.Page {
    id: root

    property int selectedListIndex: -1
    property int listsCount: 0
    property var selectedList: null

    signal createListRequested

    title: root.selectedList ? root.selectedList.label : ""

    header: QQC2.ItemDelegate {
        padding: 4
        visible: root.listsCount > 0 && root.selectedListIndex >= 0

        Kirigami.ActionTextField {
            id: newTaskField
            width: parent.width
            placeholderText: qsTr("  + New Task")
            onAccepted: {
                if (text.trim().length > 0) {
                    dummyTaskModel.append({
                        title: text.trim(),
                        description: "",
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
                isCompleted: true
            }
            ListElement {
                title: "Implement sidebar list navigation"
                description: "Add list model, item delegate, and selection handling."
                isCompleted: true
            }
            ListElement {
                title: "Add color picker dialog"
                description: "Design 10-color popup palette with custom native dialog option."
                isCompleted: true
            }
            ListElement {
                title: "Hook up task completion checkboxes"
                description: "Persist checkbox states and apply strikethrough styling."
                isCompleted: false
            }
            ListElement {
                title: "Write unit tests for task models"
                description: "Cover list creation, renaming, color changes, and task completion."
                isCompleted: false
            }
            ListElement {
                title: "Refactor Kirigami application styles"
                description: "Ensure KDE HIG adherence and dark/light Breeze theme support."
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
        }
    }
}
