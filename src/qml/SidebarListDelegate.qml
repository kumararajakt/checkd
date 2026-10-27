pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.ItemDelegate {
    id: delegateItem

    required property string modelColor
    required property string label
    required property int index

    property bool isEditing: false

    function startEditing() {
        isEditing = true;
        editField.text = delegateItem.label;
        Qt.callLater(() => {
            editField.forceActiveFocus();
            editField.selectAll();
        });
    }

    function saveEdit(newText: string) {
        newText = newText.trim();
        if (newText.length > 0) {
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegateItem.index, "label", newText);
            } else {
                delegateItem.label = newText;
            }
        }
        delegateItem.isEditing = false;
    }

    width: ListView.view ? ListView.view.width : implicitWidth

    padding: Kirigami.Units.largeSpacing

    highlighted: ListView.isCurrentItem

    onClicked: {
        ListView.view.currentIndex = delegateItem.index;
    }

    onDoubleClicked: startEditing()

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing

        // color rectange
        Rectangle {
            id: colorSwatch
            implicitWidth: 12
            implicitHeight: 12
            radius: 4
            color: delegateItem.modelColor !== "" ? delegateItem.modelColor : "blue"

            MouseArea {
                anchors.fill: parent
                anchors.margins: -4
                cursorShape: Qt.PointingHandCursor
                onDoubleClicked: {
                    colorPopup.selectedColor = delegateItem.modelColor !== "" ? delegateItem.modelColor : "#3b82f6";
                    colorPopup.open();
                }
            }
        }

        // simple space between color and label
        Item {
            implicitWidth: 4
            implicitHeight: 4
        }

        // label
        QQC2.Label {
            visible: !delegateItem.isEditing
            text: delegateItem.label
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        // text field for inline editing
        QQC2.TextField {
            id: editField
            visible: delegateItem.isEditing
            Layout.fillWidth: true
            text: delegateItem.label
            selectByMouse: true

            onAccepted: delegateItem.saveEdit(text)
            Keys.onEscapePressed: {
                text = delegateItem.label;
                delegateItem.isEditing = false;
            }
            onActiveFocusChanged: {
                if (!activeFocus && delegateItem.isEditing) {
                    delegateItem.saveEdit(text);
                }
            }
        }

        // delete icon
        QQC2.ToolButton {
            id: deleteBtn
            visible: !delegateItem.isEditing
            icon.name: "edit-delete"
            icon.width: Kirigami.Units.iconSizes.small
            icon.height: Kirigami.Units.iconSizes.small
            flat: true
            implicitWidth: Kirigami.Units.iconSizes.medium
            implicitHeight: Kirigami.Units.iconSizes.medium
            opacity: delegateItem.hovered || deleteBtn.hovered ? 1.0 : 0.5
            Behavior on opacity {
                NumberAnimation { duration: Kirigami.Units.shortDuration }
            }

            QQC2.ToolTip.text: qsTr("Delete")
            QQC2.ToolTip.visible: hovered

            onClicked: {
                if (ListView.view && ListView.view.model && typeof ListView.view.model.remove === "function") {
                    let view = ListView.view;
                    let idx = delegateItem.index;
                    view.model.remove(idx);
                    if (view.currentIndex === idx) {
                        view.currentIndex = -1;
                    } else if (view.currentIndex > idx) {
                        view.currentIndex -= 1;
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
            onTriggered: delegateItem.startEditing()
        }

        QQC2.MenuItem {
            text: qsTr("Change Color")
            icon.name: "color-picker"
            onTriggered: {
                colorPopup.selectedColor = delegateItem.modelColor !== "" ? delegateItem.modelColor : "#3b82f6";
                colorPopup.open();
            }
        }

        QQC2.MenuItem {
            text: qsTr("Delete")
            icon.name: "edit-delete"
            onTriggered: {
                if (ListView.view && ListView.view.model && typeof ListView.view.model.remove === "function") {
                    let view = ListView.view;
                    let idx = delegateItem.index;
                    view.model.remove(idx);
                    if (view.currentIndex === idx) {
                        view.currentIndex = -1;
                    } else if (view.currentIndex > idx) {
                        view.currentIndex -= 1;
                    }
                }
            }
        }
    }

    ColorPickerPopup {
        id: colorPopup
        parent: QQC2.Overlay.overlay
        x: parent ? Math.round((parent.width - width) / 2) : 0
        y: parent ? Math.round((parent.height - height) / 2) : 0
        onColorPicked: newColor => {
            if (ListView.view && ListView.view.model && typeof ListView.view.model.setProperty === "function") {
                ListView.view.model.setProperty(delegateItem.index, "modelColor", newColor);
            } else {
                delegateItem.modelColor = newColor;
            }
        }
    }
}
