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
    taskfilterproxymodel.cpp
    taskfilterproxymodel.h
    systemtray.cpp
    systemtray.h
    notificationmanager.cpp
    notificationmanager.h
    appsettings.cpp
    appsettings.h
    googleauth.cpp
    googleauth.h
    googlecalendarservice.cpp
    googlecalendarservice.h
    syncmanager.cpp
    syncmanager.h
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
    qml/GoogleSyncPage.qml
)

target_link_libraries(checkd
    PRIVATE
    Qt6::Quick
    Qt6::Qml
    Qt6::Gui
    Qt6::QuickControls2
    Qt6::Widgets
    Qt6::Sql
    Qt6::Network
    KF6::I18n
    KF6::CoreAddons
    KF6::IconThemes
    KF6::StatusNotifierItem
    KF6::Notifications
    asyncsql
)

install(TARGETS checkd ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})

# Maintain symlinks to compiled QML module for qmllint and QML language server (qmlls)
execute_process(COMMAND ${CMAKE_COMMAND} -E make_directory "${CMAKE_BINARY_DIR}/bin/org")
execute_process(COMMAND ${CMAKE_COMMAND} -E create_symlink "${CMAKE_BINARY_DIR}/bin/org" "${CMAKE_SOURCE_DIR}/org")
execute_process(COMMAND ${CMAKE_COMMAND} -E create_symlink "${CMAKE_BINARY_DIR}/bin/org" "${CMAKE_CURRENT_SOURCE_DIR}/qml/org")

add_custom_command(TARGET checkd POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E create_symlink "${CMAKE_BINARY_DIR}/bin/org" "${CMAKE_SOURCE_DIR}/org"
    COMMAND ${CMAKE_COMMAND} -E create_symlink "${CMAKE_BINARY_DIR}/bin/org" "${CMAKE_CURRENT_SOURCE_DIR}/qml/org"
    COMMENT "Updating QML module symlinks for qmllint / qmlls"
)
