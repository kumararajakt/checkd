add_executable(checkd)

ecm_add_qml_module(checkd
    URI
    org.checkd
)

target_sources(checkd
    PRIVATE
    main.cpp
    databasemanager.cpp
    databasemanager.h
    sidebarlistmodel.cpp
    sidebarlistmodel.h
    tasklistmodel.cpp
    tasklistmodel.h
    systemtray.cpp
    systemtray.h
    notificationmanager.cpp
    notificationmanager.h
    appsettings.cpp
    appsettings.h
)

target_compile_options(checkd
    PRIVATE
    -fexceptions
)

ecm_target_qml_sources(checkd
    SOURCES
    qml/Main.qml
    qml/SidebarView.qml
    qml/SidebarListDelegate.qml
    qml/ColorPickerPopup.qml
    qml/TaskListPage.qml
    qml/TaskItemDelegate.qml
    qml/RepeatDialog.qml
    qml/GeneralSettingsPage.qml
)

target_link_libraries(checkd
    PRIVATE
    Qt6::Quick
    Qt6::Qml
    Qt6::Gui
    Qt6::QuickControls2
    Qt6::Widgets
    Qt6::Sql
    KF6::I18n
    KF6::CoreAddons
    KF6::IconThemes
    KF6::StatusNotifierItem
    KF6::Notifications
    asyncsql
)

install(TARGETS checkd ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
