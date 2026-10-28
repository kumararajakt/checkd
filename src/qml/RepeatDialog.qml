pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.dateandtime as DateTime

QQC2.Popup {
    id: root

    property int dayInterval: 1
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

    implicitWidth: parent ? Math.min(parent.width - Kirigami.Units.gridUnit * 2, 400) : 400
    padding: Kirigami.Units.largeSpacing

    background: Rectangle {
        color: Kirigami.Theme.backgroundColor
        radius: Kirigami.Units.cornerRadius
        border.color: Kirigami.Theme.focusColor
        border.width: 1
    }

    contentItem: ColumnLayout {
        id: mainColumn
        spacing: Kirigami.Units.largeSpacing

        // ── Title Bar ────────────────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true

            QQC2.Label {
                text: qsTr("Repeat")
                font.bold: true
                font.pixelSize: Kirigami.Theme.defaultFont.pixelSize * 1.2
                Layout.fillWidth: true
            }

            QQC2.ToolButton {
                icon.name: "window-close"
                text: "✕"
                flat: true
                onClicked: root.close()
            }
        }

        // ── Tabs (Day, Week, Month, Year) ───────────────────────────────────
        QQC2.TabBar {
            id: tabBar
            Layout.fillWidth: true
            currentIndex: 0

            QQC2.TabButton { text: qsTr("Day") }
            QQC2.TabButton { text: qsTr("Week") }
            QQC2.TabButton { text: qsTr("Month") }
            QQC2.TabButton { text: qsTr("Year") }
        }

        // ── Tab Content ─────────────────────────────────────────────────────
        StackLayout {
            id: stackLayout
            Layout.fillWidth: true
            currentIndex: tabBar.currentIndex

            // ── Day Tab ─────────────────────────────────────────────────────
            ColumnLayout {
                id: dayTab
                spacing: Kirigami.Units.mediumSpacing
                Layout.fillWidth: true

                // Every [N] day(s)
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

                // At [Time]
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

                // Starts [Date]
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

                // ── Ends ────────────────────────────────────────────────────
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
                    onToggled: if (checked) root.endType = "never"
                }

                // Option 2: On [Date]
                RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    QQC2.RadioButton {
                        id: onRadio
                        text: qsTr("On")
                        checked: root.endType === "on"
                        QQC2.ButtonGroup.group: endsButtonGroup
                        onToggled: if (checked) root.endType = "on"
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
                        onToggled: if (checked) root.endType = "after"
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

            // ── Placeholders for Week, Month, Year ──────────────────────────
            Kirigami.PlaceholderMessage {
                text: qsTr("Weekly Repeat")
                explanation: qsTr("Configure repeat every N weeks on specific days.")
                icon.name: "view-calendar-week"
            }

            Kirigami.PlaceholderMessage {
                text: qsTr("Monthly Repeat")
                explanation: qsTr("Configure repeat every N months.")
                icon.name: "view-calendar-month"
            }

            Kirigami.PlaceholderMessage {
                text: qsTr("Yearly Repeat")
                explanation: qsTr("Configure repeat every N years.")
                icon.name: "view-calendar"
            }
        }

        // ── Footer Buttons ──────────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.smallSpacing

            Item { Layout.fillWidth: true }

            QQC2.Button {
                text: qsTr("Cancel")
                onClicked: root.close()
            }

            QQC2.Button {
                text: qsTr("Save")
                highlighted: true
                onClicked: {
                    let summary = "";
                    if (tabBar.currentIndex === 0) {
                        summary = daySpinBox.value === 1
                            ? qsTr("Daily")
                            : qsTr("Every %1 days").arg(daySpinBox.value);
                    } else if (tabBar.currentIndex === 1) {
                        summary = qsTr("Weekly");
                    } else if (tabBar.currentIndex === 2) {
                        summary = qsTr("Monthly");
                    } else {
                        summary = qsTr("Yearly");
                    }

                    if (root.repeatTime !== "") {
                        summary += " " + qsTr("at %1").arg(root.repeatTime);
                    }

                    root.repeatSaved(summary, {
                        type: tabBar.currentIndex === 0 ? "day" : (tabBar.currentIndex === 1 ? "week" : (tabBar.currentIndex === 2 ? "month" : "year")),
                        interval: daySpinBox.value,
                        time: root.repeatTime,
                        startDate: root.startDate,
                        endType: root.endType,
                        endDate: root.endDate,
                        endOccurrences: root.endOccurrences
                    });

                    root.close();
                }
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
