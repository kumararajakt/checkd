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
    required property string repeatRule
    required property string repeatConfig
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

    function getRepeatDueInfo(): var {
        let cfg = null;
        if (delegate.repeatConfig && delegate.repeatConfig.length > 0) {
            try {
                cfg = JSON.parse(delegate.repeatConfig);
            } catch (e) {
                cfg = null;
            }
        }

        let startDateStr = (cfg && cfg.startDate) ? cfg.startDate : (delegate.dueDate !== "" ? delegate.dueDate : Qt.formatDate(new Date(), "yyyy-MM-dd"));
        let timeStr = (cfg && cfg.time) ? cfg.time : delegate.dueTime;

        if (!timeStr && delegate.repeatRule) {
            let match = delegate.repeatRule.match(/\bat\s+(\d{1,2}:\d{2})/i);
            if (match && match[1]) {
                timeStr = match[1];
            }
        }

        let parts = startDateStr.split("-");
        let y = parts.length === 3 ? parseInt(parts[0], 10) : new Date().getFullYear();
        let m = parts.length === 3 ? parseInt(parts[1], 10) - 1 : new Date().getMonth();
        let d = parts.length === 3 ? parseInt(parts[2], 10) : new Date().getDate();
        let h = 23;
        let min = 59;
        if (timeStr && timeStr.length > 0) {
            let tParts = timeStr.split(":");
            if (tParts.length >= 2) {
                h = parseInt(tParts[0], 10);
                min = parseInt(tParts[1], 10);
            }
        }

        let startDt = new Date(y, m, d, h, min, 0, 0);
        let firstDue = new Date(startDt.getTime());

        if (cfg && cfg.type === "week" && Array.isArray(cfg.days) && cfg.days.length > 0) {
            for (let offset = 0; offset < 7; ++offset) {
                let testDate = new Date(startDt.getFullYear(), startDt.getMonth(), startDt.getDate() + offset, h, min, 0, 0);
                if (cfg.days.indexOf(testDate.getDay()) !== -1) {
                    firstDue = testDate;
                    break;
                }
            }
        }

        let firstDueFormattedDate = Qt.formatDate(firstDue, "yyyy-MM-dd");
        let displayText = timeStr !== "" ? (firstDueFormattedDate + "  " + timeStr) : firstDueFormattedDate;
        let overdue = !delegate.isCompleted && (firstDue.getTime() < Date.now());

        return {
            date: firstDueFormattedDate,
            time: timeStr,
            text: displayText,
            isOverdue: overdue
        };
    }

    function formatDueLabel(dateStr: string, timeStr: string): string {
        if (!dateStr || dateStr.length === 0) {
            return "";
        }
        let parts = dateStr.split("-");
        if (parts.length !== 3) {
            return timeStr !== "" ? (dateStr + "  " + timeStr) : dateStr;
        }
        let y = parseInt(parts[0], 10);
        let m = parseInt(parts[1], 10) - 1;
        let d = parseInt(parts[2], 10);

        let targetDate = new Date(y, m, d, 0, 0, 0, 0);
        let nowDate = new Date();
        let today = new Date(nowDate.getFullYear(), nowDate.getMonth(), nowDate.getDate(), 0, 0, 0, 0);

        let msPerDay = 24 * 60 * 60 * 1000;
        let diffDays = Math.round((targetDate.getTime() - today.getTime()) / msPerDay);

        let dateLabel = "";
        if (diffDays === 0) {
            dateLabel = qsTr("Today");
        } else if (diffDays === 1) {
            dateLabel = qsTr("Tomorrow");
        } else if (diffDays === -1) {
            dateLabel = qsTr("Yesterday");
        } else if (diffDays < -1) {
            let daysAgo = Math.abs(diffDays);
            if (daysAgo < 7) {
                dateLabel = qsTr("%1 days ago").arg(daysAgo);
            } else if (daysAgo < 14) {
                dateLabel = qsTr("1 week ago");
            } else {
                let weeksAgo = Math.floor(daysAgo / 7);
                dateLabel = qsTr("%1 weeks ago").arg(weeksAgo);
            }
        } else {
            dateLabel = Qt.formatDate(targetDate, "ddd, MMM d");
        }

        if (timeStr && timeStr.length > 0) {
            return dateLabel + ", " + timeStr;
        }
        return dateLabel;
    }

    function advanceToNextOccurrence() {
        let cfg = null;
        if (delegate.repeatConfig && delegate.repeatConfig.length > 0) {
            try {
                cfg = JSON.parse(delegate.repeatConfig);
            } catch (e) {
                cfg = null;
            }
        }
        let dueInfo = delegate.getRepeatDueInfo();
        let curDueDate = dueInfo.date;
        let timeStr = dueInfo.time;

        let parts = curDueDate.split("-");
        let y = parseInt(parts[0], 10);
        let m = parseInt(parts[1], 10) - 1;
        let d = parseInt(parts[2], 10);
        let cur = new Date(y, m, d);

        let nextDate = new Date(cur.getTime());
        let interval = (cfg && cfg.interval) ? cfg.interval : 1;

        if (cfg && cfg.type === "day") {
            nextDate.setDate(nextDate.getDate() + interval);
        } else if (cfg && cfg.type === "week") {
            if (Array.isArray(cfg.days) && cfg.days.length > 0) {
                let found = false;
                for (let offset = 1; offset <= 7 * interval; ++offset) {
                    let test = new Date(cur.getFullYear(), cur.getMonth(), cur.getDate() + offset);
                    if (cfg.days.indexOf(test.getDay()) !== -1) {
                        nextDate = test;
                        found = true;
                        break;
                    }
                }
                if (!found) {
                    nextDate.setDate(nextDate.getDate() + (7 * interval));
                }
            } else {
                nextDate.setDate(nextDate.getDate() + (7 * interval));
            }
        } else if (cfg && cfg.type === "month") {
            nextDate.setMonth(nextDate.getMonth() + interval);
        } else if (cfg && cfg.type === "year") {
            nextDate.setFullYear(nextDate.getFullYear() + interval);
        } else {
            nextDate.setDate(nextDate.getDate() + 1);
        }

        let formattedNextDate = Qt.formatDate(nextDate, "yyyy-MM-dd");
        if (cfg) {
            cfg.startDate = formattedNextDate;
            let cfgStr = JSON.stringify(cfg);
            delegate.repeatConfig = cfgStr;
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "repeatConfig", cfgStr);
            }
        }
        delegate.dueDate = formattedNextDate;
        if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
            ListView.view.model.setProperty(delegate.index, "dueDate", formattedNextDate);
            ListView.view.model.setProperty(delegate.index, "isCompleted", false);
        }
        checkBox.checked = false;
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
                    if (checked && delegate.repeatRule !== "") {
                        delegate.advanceToNextOccurrence();
                    } else {
                        if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                            ListView.view.model.setProperty(delegate.index, "isCompleted", checked);
                        } else {
                            delegate.isCompleted = checked;
                        }
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
                font.italic: delegate.isCompleted
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

        RowLayout {
            id: chipsRow
            visible: !delegate.isEditing && !delegate.expanded && (delegate.dueDate !== "" || delegate.repeatRule !== "")
            Layout.leftMargin: checkBox.width + arrowIcon.width + Kirigami.Units.smallSpacing * 2
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Chip {
                id: dueChip

                readonly property var repeatDueInfo: delegate.repeatRule !== "" ? delegate.getRepeatDueInfo() : null
                readonly property bool isOverdue: !delegate.isCompleted && (repeatDueInfo ? Boolean(repeatDueInfo.isOverdue) : delegate.isOverdue(delegate.dueDate, delegate.dueTime))

                text: {
                    if (delegate.repeatRule !== "" && repeatDueInfo) {
                        return delegate.formatDueLabel(repeatDueInfo.date, repeatDueInfo.time);
                    } else if (delegate.dueDate !== "") {
                        return delegate.formatDueLabel(delegate.dueDate, delegate.dueTime);
                    }
                    return "";
                }

                icon.name: delegate.repeatRule !== "" ? "media-playlist-repeat" : "view-calendar"
                icon.color: isOverdue ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
                labelItem.color: isOverdue ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor

                interactive: false

                QQC2.ToolTip.text: delegate.repeatRule !== "" ? delegate.repeatRule : ""
                QQC2.ToolTip.visible: delegate.repeatRule !== "" && dueChipHover.hovered

                HoverHandler {
                    id: dueChipHover
                }

                background: Rectangle {
                    radius: Kirigami.Units.cornerRadius
                    color: dueChip.isOverdue ? Kirigami.Theme.negativeBackgroundColor : "transparent"
                    border.color: dueChip.isOverdue ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.alternateBackgroundColor
                    border.width: 1
                }
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


            RowLayout {
                spacing: Kirigami.Units.smallSpacing

                QQC2.Button {
                    id: repeatBtn
                    text: delegate.repeatRule !== "" ? delegate.repeatRule : qsTr("Repeat")
                    icon.name: "media-playlist-repeat"
                    onClicked: {
                        if (delegate.repeatConfig && delegate.repeatConfig.length > 0) {
                            try {
                                repeatDialog.loadConfig(JSON.parse(delegate.repeatConfig));
                            } catch (e) {}
                        }
                        repeatDialog.open();
                    }
                }

                QQC2.ToolButton {
                    visible: delegate.repeatRule !== ""
                    icon.name: "edit-clear"
                    flat: true
                    QQC2.ToolTip.text: qsTr("Clear Repeat")
                    QQC2.ToolTip.visible: hovered
                    onClicked: {
                        delegate.repeatRule = "";
                        delegate.repeatConfig = "";
                        if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                            ListView.view.model.setProperty(delegate.index, "repeatRule", "");
                            ListView.view.model.setProperty(delegate.index, "repeatConfig", "");
                        }
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
        x: parent ? Math.round((parent.width - width) / 2) : 0
        y: parent ? Math.round((parent.height - height) / 2) : 0
        resettable: true
        autoAccept: true
        onAccepted: {
            let d = datePopup.value;
            let formatted = Qt.formatDate(d, "yyyy-MM-dd");
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "dueDate", formatted);
            } else {
                delegate.dueDate = formatted;
            }
            datePopup.close();
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
        x: parent ? Math.round((parent.width - width) / 2) : 0
        y: parent ? Math.round((parent.height - height) / 2) : 0
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

    RepeatDialog {
        id: repeatDialog
        parent: QQC2.Overlay.overlay
        x: parent ? Math.round((parent.width - width) / 2) : 0
        y: parent ? Math.round((parent.height - height) / 2) : 0
        onRepeatSaved: (summary, config) => {
            delegate.repeatRule = summary;
            let cfgStr = JSON.stringify(config);
            delegate.repeatConfig = cfgStr;
            if (config && config.startDate) {
                delegate.dueDate = config.startDate;
            }
            if (config && config.time) {
                delegate.dueTime = config.time;
            }
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegate.index, "repeatRule", summary);
                ListView.view.model.setProperty(delegate.index, "repeatConfig", cfgStr);
                if (config && config.startDate) {
                    ListView.view.model.setProperty(delegate.index, "dueDate", config.startDate);
                }
                if (config && config.time) {
                    ListView.view.model.setProperty(delegate.index, "dueTime", config.time);
                }
            }
        }
    }
}
