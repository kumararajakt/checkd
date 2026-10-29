pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigamiaddons.formcard as FormCard
import org.checkd

FormCard.FormCardPage {
    id: root

    title: qsTr("General")

    FormCard.FormHeader {
        title: qsTr("Application")
    }

    FormCard.FormCard {
        FormCard.FormSwitchDelegate {
            id: closeToTraySwitch
            text: qsTr("Close to tray")
            description: qsTr("Keep the application running in the system tray when closing the window")
            checked: AppSettings.closeToTray
            onToggled: {
                AppSettings.closeToTray = checked;
            }
        }

        FormCard.FormDelegateSeparator {}

        FormCard.FormSwitchDelegate {
            id: autostartSwitch
            text: qsTr("Autostart")
            description: qsTr("Start CheckD automatically on system startup")
            checked: AppSettings.autostart
            onToggled: {
                AppSettings.autostart = checked;
            }
        }
    }
}
