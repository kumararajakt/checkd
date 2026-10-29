pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard
import org.checkd

FormCard.FormCardPage {
    id: root

    title: qsTr("Google Sync")

    FormCard.FormHeader {
        title: qsTr("Google Account")
    }

    FormCard.FormCard {
        FormCard.FormTextDelegate {
            text: qsTr("Status")
            description: SyncManager.signedIn
                         ? (SyncManager.accountEmail !== "" ? SyncManager.accountEmail : qsTr("Connected"))
                         : qsTr("Not connected")
        }

        FormCard.FormDelegateSeparator {
            visible: SyncManager.signedIn && SyncManager.lastSyncTime !== ""
        }

        FormCard.FormTextDelegate {
            visible: SyncManager.signedIn && SyncManager.lastSyncTime !== ""
            text: qsTr("Last Synced")
            description: SyncManager.lastSyncTime
        }

        FormCard.FormDelegateSeparator {
            visible: SyncManager.lastError !== ""
        }

        FormCard.FormTextDelegate {
            visible: SyncManager.lastError !== ""
            text: qsTr("Status Message")
            description: SyncManager.lastError
        }
    }

    FormCard.FormHeader {
        title: qsTr("Sync Controls")
    }

    FormCard.FormCard {
        FormCard.FormButtonDelegate {
            visible: !SyncManager.signedIn
            text: qsTr("Sign in with Google")
            icon.name: "im-google"
            onClicked: {
                SyncManager.signIn();
            }
        }

        FormCard.FormButtonDelegate {
            visible: SyncManager.signedIn
            text: SyncManager.syncing ? qsTr("Syncing…") : qsTr("Sync Now")
            icon.name: "view-refresh"
            enabled: !SyncManager.syncing
            onClicked: {
                SyncManager.sync();
            }
        }

        FormCard.FormDelegateSeparator {
            visible: SyncManager.signedIn
        }

        FormCard.FormButtonDelegate {
            visible: SyncManager.signedIn
            text: qsTr("Sign Out")
            icon.name: "system-log-out"
            onClicked: {
                SyncManager.signOut();
            }
        }
    }
}
