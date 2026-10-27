pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.ItemDelegate {
    id: delegate

    required property string title
    required property string description
    required property bool isCompleted
    required property int index

    property bool expanded: false

    width: ListView.view ? ListView.view.width : implicitWidth

    onClicked: {
        expanded = !expanded;
    }

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
                text: delegate.title
                Layout.fillWidth: true
                elide: Text.ElideRight
                font.strikeout: delegate.isCompleted
                opacity: delegate.isCompleted ? 0.6 : 1.0
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

            QQC2.Button {
                text: qsTr("Set Due Date")
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
}
