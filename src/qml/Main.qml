import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root

    width: Kirigami.Settings.isMobile ? -1 : Kirigami.Units.gridUnit * 65
    height: Kirigami.Settings.isMobile ? -1 : Kirigami.Units.gridUnit * 45

    title: i18nc("@title:window", "CheckD")



    pageStack.initialPage: Kirigami.Page {
        Controls.Label {
            anchors.centerIn: parent
            text: i18n("Hello World!")
        }
    }
}
