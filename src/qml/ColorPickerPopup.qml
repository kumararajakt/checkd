pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Dialogs as Dialogs
import org.kde.kirigami as Kirigami

Kirigami.Dialog {
    id: root

    property string selectedColor: "#3b82f6"
    signal colorPicked(string color)

    readonly property var presetColors: [
        "#3b82f6", // Blue
        "#ef4444", // Red
        "#22c55e", // Green
        "#eab308", // Yellow
        "#f97316", // Orange
        "#ec4899", // Pink
        "#8b5cf6", // Purple
        "#14b8a6", // Teal
        "#64748b", // Slate
        "#1e293b"  // Dark charcoal
    ]

    modal: true
    focus: true
    dim: true
    closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutside

    implicitWidth: 260
    // implicitHeight: contentColumn.implicitHeight + topPadding + bottomPadding
    padding: 16

    title: qsTr("List color")



    Dialogs.ColorDialog {
        id: nativeColorDialog
        title: qsTr("Select Custom Color")
        selectedColor: root.selectedColor
        onAccepted: {
            root.selectedColor = selectedColor.toString();
            root.colorPicked(root.selectedColor);
            root.close();
        }
    }

    contentItem: ColumnLayout {
        id: contentColumn
        spacing: 16


        // ── Preset Color Swatches (2 rows x 5 columns) ──────────────────────
        GridLayout {
            columns: 5
            columnSpacing: 10
            rowSpacing: 10
            Layout.alignment: Qt.AlignHCenter

            Repeater {
                model: root.presetColors

                delegate: Item {
                    id: swatchItem
                    required property string modelData
                    required property int index

                    implicitWidth: 32
                    implicitHeight: 32

                    readonly property bool isSelected: root.selectedColor.toLowerCase() === swatchItem.modelData.toLowerCase()

                    // Selection Ring
                    Rectangle {
                        anchors.fill: parent
                        radius: width / 2
                        color: "transparent"
                        border.color: swatchItem.isSelected ? "#ffffff" : "transparent"
                        border.width: 2
                        visible: swatchItem.isSelected
                    }

                    // Inner Color Circle
                    Rectangle {
                        anchors.centerIn: parent
                        width: swatchItem.isSelected ? 24 : 28
                        height: width
                        radius: width / 2
                        color: swatchItem.modelData
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selectedColor = swatchItem.modelData;
                            root.colorPicked(swatchItem.modelData);
                            root.close();
                        }
                    }
                }
            }
        }

        // ── Custom Color Row ────────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 4

            QQC2.Label {
                text: qsTr("Custom")
                color: "#9ca3af"
                font.pixelSize: 14
                Layout.fillWidth: true
            }

            Rectangle {
                id: customSwatch
                implicitWidth: 26
                implicitHeight: 26
                radius: 4
                color: root.selectedColor
                border.color: "#374151"
                border.width: 1

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        nativeColorDialog.selectedColor = root.selectedColor;
                        nativeColorDialog.open();
                    }
                }
            }
        }
    }
}
