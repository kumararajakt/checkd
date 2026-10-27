pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: sidebarView

    title: ""

    // ── Add list ─────────────────────────────────────────────────────────

    header: Kirigami.AbstractApplicationHeader {

        QQC2.ItemDelegate {
            width: parent.width

            contentItem: Kirigami.ActionTextField {
                id: addListField
                placeholderText: qsTr("New List")
                Layout.fillWidth: true

                leftActions: [
                    Kirigami.Action {
                        icon.name: "list-add"
                        onTriggered: addListField.accepted()
                    }
                ]

                onAccepted: {
                    if (text.trim().length > 0) {
                        dummyModel.append({
                            label: text.trim(),
                            modelColor: "#3daee9"
                        });
                        text = "";
                    }
                }
            }
        }
    }

    // ── Sidebar List ─────────────────────────────────────────────────────────
    ListView {
        id: navList

        anchors.fill: parent

        model: ListModel {
            id: dummyModel

            ListElement {
                label: "Inbox"
                modelColor: "#3daee9"
            }
            ListElement {
                label: "Personal"
                modelColor: "#2ecc71"
            }
            ListElement {
                label: "Work"
                modelColor: "#e74c3c"
            }
            ListElement {
                label: "Shopping"
                modelColor: "#f39c12"
            }
            ListElement {
                label: "Projects"
                modelColor: "#9b59b6"
            }
            ListElement {
                label: "Reading List"
                modelColor: "#1abc9c"
            }
        }

        delegate: SidebarListDelegate {}
    }
}
