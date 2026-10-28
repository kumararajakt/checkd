pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.dateandtime as DateTime

QQC2.ItemDelegate {
    id: delegate

    required property string title
    required property string description
    required property string dueDate
    required property string dueTime
    required property bool isCompleted
    required property int index

    property bool expanded: false
    property bool isEditing: false

    function startEditing() {
        isEditing = true;
        editField.text = delegate.title;
        Qt.callLater(() => {
            editField.forceActiveFocus();
            editField.selectAll();
        });
    }

    function saveEdit(newText: string) {
        newText = newText.trim();
        if (newText.length > 0) {
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "title", newText);
            } else {
                delegate.title = newText;
            }
        }
        delegate.isEditing = false;
    }

    function isOverdue(dueDateStr: string, dueTimeStr: string): bool {
        if (!dueDateStr || dueDateStr.length === 0) {
            return false;
        }
        let parts = dueDateStr.split("-");
        if (parts.length !== 3) {
            return false;
        }
        let year = parseInt(parts[0], 10);
        let month = parseInt(parts[1], 10) - 1;
        let day = parseInt(parts[2], 10);
        let hours = 23;
        let minutes = 59;
        if (dueTimeStr && dueTimeStr.length > 0) {
            let tParts = dueTimeStr.split(":");
            if (tParts.length >= 2) {
                hours = parseInt(tParts[0], 10);
                minutes = parseInt(tParts[1], 10);
            }
        }
        let dueDateTime = new Date(year, month, day, hours, minutes, 0);
        return dueDateTime.getTime() < Date.now();
    }

    width: ListView.view ? ListView.view.width : implicitWidth

    onClicked: {
        if (!isEditing) {
            expanded = !expanded;
        }
    }

    onDoubleClicked: startEditing()

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        // ── Main Header Row ─────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            QQC2.CheckBox {
                id: checkBox
                checked: delegate.isCompleted
                onToggled: {
                    if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                        ListView.view.model.setProperty(delegate.index, "isCompleted", checked);
                    }
                }
            }

            Kirigami.Icon {
                id: arrowIcon
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: Kirigami.Units.iconSizes.small
                source: "go-next"
                rotation: delegate.expanded ? 90 : 0
                opacity: 0.7

                Behavior on rotation {
                    NumberAnimation {
                        duration: Kirigami.Units.shortDuration
                    }
                }
            }

            QQC2.Label {
                visible: !delegate.isEditing
                text: delegate.title
                Layout.fillWidth: true
                elide: Text.ElideRight
                font.strikeout: delegate.isCompleted
                opacity: delegate.isCompleted ? 0.6 : 1.0
            }



            QQC2.TextField {
                id: editField
                visible: delegate.isEditing
                Layout.fillWidth: true
                text: delegate.title
                selectByMouse: true

                onAccepted: delegate.saveEdit(text)
                Keys.onEscapePressed: {
                    text = delegate.title;
                    delegate.isEditing = false;
                }
                onActiveFocusChanged: {
                    if (!activeFocus && delegate.isEditing) {
                        delegate.saveEdit(text);
                    }
                }
            }
        }

        Kirigami.Chip {
            id: dueChip
            visible: !delegate.isEditing && delegate.dueDate !== ""
            Layout.leftMargin: checkBox.width + arrowIcon.width + Kirigami.Units.smallSpacing * 2

            readonly property bool isOverdue: !delegate.isCompleted && delegate.isOverdue(delegate.dueDate, delegate.dueTime)

            text: delegate.dueTime !== "" ? (delegate.dueDate + "  " + delegate.dueTime) : delegate.dueDate
            icon.name: "view-calendar"
            icon.color: isOverdue ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
            labelItem.color: isOverdue ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor

            interactive: false

            background: Rectangle {
                radius: Kirigami.Units.cornerRadius
                border.color: Kirigami.Theme.alternateBackgroundColor
                border.width: 1
            }
        }

        // ── Accordion Description Content ───────────────────────────────
        ColumnLayout {
            id: descContainer
            visible: delegate.expanded
            Layout.fillWidth: true
            Layout.leftMargin: checkBox.width + arrowIcon.width + Kirigami.Units.smallSpacing * 2
            Layout.rightMargin: Kirigami.Units.smallSpacing
            Layout.bottomMargin: Kirigami.Units.smallSpacing
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Separator {}

            RowLayout {
                spacing: Kirigami.Units.smallSpacing

                QQC2.Button {
                    id: dueDateButton
                    icon.name: "view-calendar"
                    text: delegate.dueDate !== "" ? qsTr("Due: %1").arg(delegate.dueDate) : qsTr("Set Due Date")
                    onClicked: {
                        if (delegate.dueDate !== "") {
                            datePopup.value = new Date(delegate.dueDate);
                        } else {
                            datePopup.value = new Date();
                        }
                        datePopup.open();
                    }
                }

                QQC2.Button {
                    id: dueTimeButton
                    visible: delegate.dueDate !== ""
                    icon.name: "preferences-system-time"
                    text: delegate.dueTime !== "" ? delegate.dueTime : qsTr("Set Time")
                    onClicked: {
                        let now = new Date();
                        if (delegate.dueTime !== "") {
                            let parts = delegate.dueTime.split(":");
                            if (parts.length >= 2) {
                                now.setHours(parseInt(parts[0], 10), parseInt(parts[1], 10), 0);
                            }
                        }
                        timePopup.value = now;
                        timePopup.open();
                    }
                }

                QQC2.ToolButton {
                    visible: delegate.dueDate !== "" && delegate.dueTime !== ""
                    icon.name: "edit-clear"
                    flat: true
                    QQC2.ToolTip.text: qsTr("Clear Time")
                    QQC2.ToolTip.visible: hovered
                    onClicked: {
                        if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                            ListView.view.model.setProperty(delegate.index, "dueTime", "");
                        } else {
                            delegate.dueTime = "";
                        }
                    }
                }

                QQC2.ToolButton {
                    visible: delegate.dueDate !== ""
                    icon.name: "edit-delete"
                    flat: true
                    QQC2.ToolTip.text: qsTr("Clear Due Date")
                    QQC2.ToolTip.visible: hovered
                    onClicked: {
                        if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                            ListView.view.model.setProperty(delegate.index, "dueDate", "");
                            ListView.view.model.setProperty(delegate.index, "dueTime", "");
                        } else {
                            delegate.dueDate = "";
                            delegate.dueTime = "";
                        }
                    }
                }
            }

            Kirigami.Separator {}

            QQC2.TextArea {
                id: descTextArea
                Layout.fillWidth: true
                text: delegate.description
                placeholderText: qsTr("Add description…")
                wrapMode: TextEdit.WordWrap
                selectByMouse: true

                onEditingFinished: {
                    if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                        ListView.view.model.setProperty(delegate.index, "description", text);
                    }
                }
            }
        }
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: contextMenu.popup()
    }

    QQC2.Menu {
        id: contextMenu

        QQC2.MenuItem {
            text: qsTr("Edit")
            icon.name: "document-edit"
            onTriggered: delegate.startEditing()
        }

        QQC2.MenuItem {
            text: qsTr("Delete")
            icon.name: "edit-delete"
            onTriggered: {
                if (ListView.view && ListView.view.model && typeof ListView.view.model.remove === "function") {
                    ListView.view.model.remove(delegate.index);
                }
            }
        }
    }

    DateTime.DatePopup {
        id: datePopup
        parent: QQC2.Overlay.overlay
        resettable: true
        onAccepted: {
            let d = datePopup.value;
            let formatted = Qt.formatDate(d, "yyyy-MM-dd");
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "dueDate", formatted);
            } else {
                delegate.dueDate = formatted;
            }
        }
        onReset: {
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "dueDate", "");
            } else {
                delegate.dueDate = "";
            }
        }
    }

    DateTime.TimePopup {
        id: timePopup
        parent: QQC2.Overlay.overlay
        resettable: true
        onAccepted: {
            let t = timePopup.value;
            let formattedTime = Qt.formatTime(t, "hh:mm");
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "dueTime", formattedTime);
            } else {
                delegate.dueTime = formattedTime;
            }
        }
        onReset: {
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "dueTime", "");
            } else {
                delegate.dueTime = "";
            }
        }
    }
}
