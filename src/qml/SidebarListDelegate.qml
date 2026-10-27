import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.ItemDelegate {
    id: delegateItem

    required property string modelColor
    required property string label

    width: ListView.view ? ListView.view.width : implicitWidth

    leftPadding: Kirigami.Units.largeSpacing
    rightPadding: Kirigami.Units.smallSpacing

    onDoubleClicked: {}

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing

        Rectangle {
            implicitWidth: 12
            implicitHeight: 12
            radius: 4
            color: delegateItem.modelColor !== "" ? delegateItem.modelColor : 'blue'
        }

        Item {
            implicitWidth: 4
            implicitHeight: 4
        }

        QQC2.Label {
            text: delegateItem.label
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Item {
            Layout.fillWidth: true
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
        }

        QQC2.MenuItem {
            text: qsTr("Delete")
            icon.name: "edit-delete"
        }
    }
}
