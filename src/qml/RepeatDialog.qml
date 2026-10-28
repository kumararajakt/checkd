pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.dateandtime as DateTime

Kirigami.Dialog {
    id: root

    property int dayInterval: 1
    property int weekInterval: 1
    property var selectedDays: [new Date().getDay()]
    property int monthInterval: 1
    property string monthType: "day" // "day" | "the"
    property var monthDay: new Date().getDate()
    property int monthOrdinalIndex: 0
    property int monthWeekdayIndex: new Date().getDay()
    property string repeatTime: "12:00"
    property string startDate: Qt.formatDate(new Date(), "yyyy-MM-dd")
    property string endType: "never" // "never" | "on" | "after"
    property string endDate: Qt.formatDate(new Date(), "yyyy-MM-dd")
    property int endOccurrences: 1

    signal repeatSaved(string summary, var config)

    modal: true
    focus: true
    dim: true
    closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutside

    implicitWidth: parent ? Math.min(parent.width - Kirigami.Units.gridUnit * 2, 430) : 430
    padding: Kirigami.Units.largeSpacing

    title: qsTr("Repeat")

    // ── Footer Buttons ──────────────────────────────────────────────────
    footer: RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: Kirigami.Units.smallSpacing

        Item {
            Layout.fillWidth: true
        }

        QQC2.Button {
            text: qsTr("Cancel")
            onClicked: root.close()
        }

        QQC2.Button {
            text: qsTr("Save")
            highlighted: true
            onClicked: {
                let summary = "";
                let config = {
                    time: root.repeatTime,
                    startDate: root.startDate,
                    endType: root.endType,
                    endDate: root.endDate,
                    endOccurrences: root.endOccurrences
                };

                if (tabBar.currentIndex === 0) {
                    config.type = "day";
                    config.interval = daySpinBox.value;
                    summary = daySpinBox.value === 1 ? qsTr("Daily") : qsTr("Every %1 days").arg(daySpinBox.value);
                } else if (tabBar.currentIndex === 1) {
                    config.type = "week";
                    config.interval = weekSpinBox.value;
                    let dayNames = [qsTr("Sun"), qsTr("Mon"), qsTr("Tue"), qsTr("Wed"), qsTr("Thu"), qsTr("Fri"), qsTr("Sat")];
                    let sortedDays = [...root.selectedDays].sort((a, b) => a - b);
                    config.days = sortedDays;
                    let daysStr = sortedDays.map(d => dayNames[d]).join(", ");
                    if (weekSpinBox.value === 1) {
                        summary = daysStr ? qsTr("Weekly on %1").arg(daysStr) : qsTr("Weekly");
                    } else {
                        summary = daysStr ? qsTr("Every %1 weeks on %2").arg(weekSpinBox.value).arg(daysStr) : qsTr("Every %1 weeks").arg(weekSpinBox.value);
                    }
                } else if (tabBar.currentIndex === 2) {
                    config.type = "month";
                    config.interval = monthSpinBox.value;
                    config.monthType = root.monthType;
                    if (root.monthType === "day") {
                        config.day = root.monthDay;
                        let isLastDay = (monthDayCombo.currentIndex === 31 || root.monthDay === qsTr("last day") || root.monthDay === "last day");
                        if (isLastDay) {
                            if (monthSpinBox.value === 1) {
                                summary = qsTr("Monthly on the last day");
                            } else {
                                summary = qsTr("Every %1 months on the last day").arg(monthSpinBox.value);
                            }
                        } else {
                            if (monthSpinBox.value === 1) {
                                summary = qsTr("Monthly on day %1").arg(root.monthDay);
                            } else {
                                summary = qsTr("Every %1 months on day %2").arg(monthSpinBox.value).arg(root.monthDay);
                            }
                        }
                    } else {
                        config.ordinal = monthOrdinalCombo.currentText;
                        config.weekday = monthWeekdayCombo.currentText;
                        if (monthSpinBox.value === 1) {
                            summary = qsTr("Monthly on the %1 %2").arg(config.ordinal).arg(config.weekday);
                        } else {
                            summary = qsTr("Every %1 months on the %2 %3").arg(monthSpinBox.value).arg(config.ordinal).arg(config.weekday);
                        }
                    }
                } else {
                    config.type = "year";
                    summary = qsTr("Yearly");
                }

                if (root.repeatTime !== "") {
                    summary += " " + qsTr("at %1").arg(root.repeatTime);
                }

                root.repeatSaved(summary, config);
                root.close();
            }
        }
    }

    contentItem: ColumnLayout {
        id: mainColumn
        spacing: Kirigami.Units.mediumSpacing

        // ── Tabs (Day, Week, Month, Year) ───────────────────────────────────
        QQC2.TabBar {
            id: tabBar
            Layout.fillWidth: true
            currentIndex: 0

            QQC2.TabButton {
                text: qsTr("Day")
            }
            QQC2.TabButton {
                text: qsTr("Week")
            }
            QQC2.TabButton {
                text: qsTr("Month")
            }
            QQC2.TabButton {
                text: qsTr("Year")
            }
        }

        // ── Tab-specific Content ────────────────────────────────────────────
        StackLayout {
            id: stackLayout
            Layout.fillWidth: true
            currentIndex: tabBar.currentIndex

            // ── Day Tab ─────────────────────────────────────────────────────
            ColumnLayout {
                id: dayTab
                spacing: Kirigami.Units.mediumSpacing
                Layout.fillWidth: true

                RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.Label {
                        text: qsTr("Every")
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                    }

                    QQC2.SpinBox {
                        id: daySpinBox
                        from: 1
                        to: 365
                        value: root.dayInterval
                        editable: true
                        onValueModified: root.dayInterval = value
                    }

                    QQC2.Label {
                        text: daySpinBox.value === 1 ? qsTr("day") : qsTr("days")
                    }
                }
            }

            // ── Week Tab ────────────────────────────────────────────────────
            ColumnLayout {
                id: weekTab
                spacing: Kirigami.Units.mediumSpacing
                Layout.fillWidth: true

                // "On" Weekdays
                ColumnLayout {
                    spacing: Kirigami.Units.smallSpacing
                    Layout.fillWidth: true

                    QQC2.Label {
                        text: qsTr("On")
                        font.bold: true
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Repeater {
                            model: [
                                {
                                    name: qsTr("Sun"),
                                    day: 0
                                },
                                {
                                    name: qsTr("Mon"),
                                    day: 1
                                },
                                {
                                    name: qsTr("Tue"),
                                    day: 2
                                },
                                {
                                    name: qsTr("Wed"),
                                    day: 3
                                },
                                {
                                    name: qsTr("Thu"),
                                    day: 4
                                },
                                {
                                    name: qsTr("Fri"),
                                    day: 5
                                },
                                {
                                    name: qsTr("Sat"),
                                    day: 6
                                }
                            ]

                            delegate: QQC2.Button {
                                id: dayBtn
                                required property var modelData
                                required property int index

                                Layout.fillWidth: true
                                text: dayBtn.modelData.name
                                checkable: true
                                checked: root.selectedDays.indexOf(dayBtn.modelData.day) !== -1

                                onClicked: {
                                    let list = [...root.selectedDays];
                                    let d = dayBtn.modelData.day;
                                    let idx = list.indexOf(d);
                                    if (checked && idx === -1) {
                                        list.push(d);
                                    } else if (!checked && idx !== -1) {
                                        list.splice(idx, 1);
                                    }
                                    root.selectedDays = list;
                                }
                            }
                        }
                    }
                }

                // Every [N] week(s)
                RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.Label {
                        text: qsTr("Every")
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                    }

                    QQC2.SpinBox {
                        id: weekSpinBox
                        from: 1
                        to: 52
                        value: root.weekInterval
                        editable: true
                        onValueModified: root.weekInterval = value
                    }

                    QQC2.Label {
                        text: weekSpinBox.value === 1 ? qsTr("week") : qsTr("weeks")
                    }
                }
            }

            // ── Month Tab ───────────────────────────────────────────────────
            ColumnLayout {
                id: monthTab
                spacing: Kirigami.Units.mediumSpacing
                Layout.fillWidth: true

                QQC2.ButtonGroup {
                    id: monthTypeGroup
                }

                // Option 1: On day [1..31]
                RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.RadioButton {
                        id: onDayRadio
                        text: qsTr("On day")
                        checked: root.monthType === "day"
                        QQC2.ButtonGroup.group: monthTypeGroup
                        Layout.preferredWidth: Math.max(onDayRadio.implicitWidth, onTheRadio.implicitWidth)
                        onToggled: if (checked)
                            root.monthType = "day"
                    }

                    QQC2.ComboBox {
                        id: monthDayCombo
                        enabled: onDayRadio.checked
                        model: ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", qsTr("last day")]
                        currentIndex: {
                            if (root.monthDay === qsTr("last day") || root.monthDay === "last day") {
                                return 31;
                            }
                            let n = parseInt(root.monthDay, 10);
                            return isNaN(n) ? 0 : Math.max(0, Math.min(30, n - 1));
                        }
                        onActivated: root.monthDay = currentText
                    }
                }

                // Option 2: On the [first..last] [Sunday..weekend day]
                RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.RadioButton {
                        id: onTheRadio
                        text: qsTr("On the")
                        checked: root.monthType === "the"
                        QQC2.ButtonGroup.group: monthTypeGroup
                        Layout.preferredWidth: Math.max(onDayRadio.implicitWidth, onTheRadio.implicitWidth)
                        onToggled: if (checked)
                            root.monthType = "the"
                    }

                    QQC2.ComboBox {
                        id: monthOrdinalCombo
                        enabled: onTheRadio.checked
                        model: [qsTr("first"), qsTr("second"), qsTr("third"), qsTr("fourth"), qsTr("last")]
                        currentIndex: root.monthOrdinalIndex
                        onActivated: root.monthOrdinalIndex = currentIndex
                    }

                    QQC2.ComboBox {
                        id: monthWeekdayCombo
                        enabled: onTheRadio.checked
                        model: [qsTr("Sunday"), qsTr("Monday"), qsTr("Tuesday"), qsTr("Wednesday"), qsTr("Thursday"), qsTr("Friday"), qsTr("Saturday")]
                        currentIndex: Math.max(0, Math.min(6, root.monthWeekdayIndex))
                        onActivated: root.monthWeekdayIndex = currentIndex
                    }
                }

                // Every [N] month(s)
                RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.Label {
                        text: qsTr("Every")
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 3
                    }

                    QQC2.SpinBox {
                        id: monthSpinBox
                        from: 1
                        to: 120
                        value: root.monthInterval
                        editable: true
                        onValueModified: root.monthInterval = value
                    }

                    QQC2.Label {
                        text: monthSpinBox.value === 1 ? qsTr("month") : qsTr("months")
                    }
                }
            }

            Kirigami.PlaceholderMessage {
                text: qsTr("Yearly Repeat")
                explanation: qsTr("Configure repeat every N years.")
                icon.name: "view-calendar"
            }
        }

        // ── Common: At [Time] ───────────────────────────────────────────────
        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            QQC2.Label {
                text: qsTr("At")
                Layout.preferredWidth: Kirigami.Units.gridUnit * 3
            }

            QQC2.Button {
                id: timeBtn
                icon.name: "preferences-system-time"
                text: root.repeatTime !== "" ? root.repeatTime : qsTr("Set Time")
                onClicked: {
                    let now = new Date();
                    if (root.repeatTime !== "") {
                        let parts = root.repeatTime.split(":");
                        if (parts.length >= 2) {
                            now.setHours(parseInt(parts[0], 10), parseInt(parts[1], 10), 0);
                        }
                    }
                    repeatTimePopup.value = now;
                    repeatTimePopup.open();
                }
            }
        }

        // ── Common: Starts [Date] ───────────────────────────────────────────
        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            QQC2.Label {
                text: qsTr("Starts")
                Layout.preferredWidth: Kirigami.Units.gridUnit * 3
            }

            QQC2.Button {
                id: startBtn
                icon.name: "view-calendar"
                text: root.startDate
                onClicked: {
                    if (root.startDate !== "") {
                        startDatePopup.value = new Date(root.startDate);
                    }
                    startDatePopup.open();
                }
            }
        }

        // ── Common: Ends ────────────────────────────────────────────────────
        QQC2.Label {
            text: qsTr("Ends")
            font.bold: true
            Layout.topMargin: Kirigami.Units.smallSpacing
        }

        QQC2.ButtonGroup {
            id: endsButtonGroup
        }

        // Option 1: Never
        QQC2.RadioButton {
            id: neverRadio
            text: qsTr("Never")
            checked: root.endType === "never"
            QQC2.ButtonGroup.group: endsButtonGroup
            onToggled: if (checked)
                root.endType = "never"
        }

        // Option 2: On [Date]
        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            QQC2.RadioButton {
                id: onRadio
                text: qsTr("On")
                checked: root.endType === "on"
                QQC2.ButtonGroup.group: endsButtonGroup
                onToggled: if (checked)
                    root.endType = "on"
            }

            QQC2.Button {
                enabled: onRadio.checked
                icon.name: "view-calendar"
                text: root.endDate
                onClicked: {
                    if (root.endDate !== "") {
                        endDatePopup.value = new Date(root.endDate);
                    }
                    endDatePopup.open();
                }
            }
        }

        // Option 3: After [N] occurrences
        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            QQC2.RadioButton {
                id: afterRadio
                text: qsTr("After")
                checked: root.endType === "after"
                QQC2.ButtonGroup.group: endsButtonGroup
                onToggled: if (checked)
                    root.endType = "after"
            }

            QQC2.SpinBox {
                id: occurrencesSpinBox
                enabled: afterRadio.checked
                from: 1
                to: 999
                value: root.endOccurrences
                editable: true
                onValueModified: root.endOccurrences = value
            }

            QQC2.Label {
                enabled: afterRadio.checked
                text: occurrencesSpinBox.value === 1 ? qsTr("occurrence") : qsTr("occurrences")
            }
        }
    }

    // ── Nested Popups ───────────────────────────────────────────────────────
    DateTime.TimePopup {
        id: repeatTimePopup
        parent: QQC2.Overlay.overlay
        onAccepted: {
            root.repeatTime = Qt.formatTime(repeatTimePopup.value, "hh:mm");
        }
    }

    DateTime.DatePopup {
        id: startDatePopup
        parent: QQC2.Overlay.overlay
        onAccepted: {
            root.startDate = Qt.formatDate(startDatePopup.value, "yyyy-MM-dd");
        }
    }

    DateTime.DatePopup {
        id: endDatePopup
        parent: QQC2.Overlay.overlay
        onAccepted: {
            root.endDate = Qt.formatDate(endDatePopup.value, "yyyy-MM-dd");
        }
    }
}
