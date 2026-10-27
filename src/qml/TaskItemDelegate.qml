pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.ItemDelegate {
    id: delegate

    required property string title
    required property bool isCompleted
    required property int index

    width: ListView.view ? ListView.view.width : implicitWidth

    contentItem: RowLayout {
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

        QQC2.Label {
            text: delegate.title
            Layout.fillWidth: true
            elide: Text.ElideRight
            font.strikeout: delegate.isCompleted
            opacity: delegate.isCompleted ? 0.6 : 1.0
        }
    }
}
