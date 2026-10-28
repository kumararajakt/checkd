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
}
