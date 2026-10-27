add_executable(checkd)

ecm_add_qml_module(checkd
    URI
    org.checkd
)

target_sources(checkd
    PRIVATE
    main.cpp
)

ecm_target_qml_sources(checkd
    SOURCES
    qml/Main.qml
)

target_link_libraries(checkd
    PRIVATE
    Qt6::Quick
    Qt6::Qml
    Qt6::Gui
    Qt6::QuickControls2
    Qt6::Widgets
    KF6::I18n
    KF6::CoreAddons
    KF6::IconThemes
)

install(TARGETS checkd ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
