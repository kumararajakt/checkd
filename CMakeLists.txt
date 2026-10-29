cmake_minimum_required(VERSION 3.20)
project(checkd)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(ECM 6.0.0 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)
include(ECMFindQmlModule)
include(ECMQmlModule)

find_package(Qt6 REQUIRED COMPONENTS
    Core
    Quick
    Test
    Gui
    QuickControls2
    Widgets
    Sql
    Network
)

find_package(KF6 REQUIRED COMPONENTS
    Kirigami
    I18n
    CoreAddons
    QQC2DesktopStyle
    IconThemes
    StatusNotifierItem
    Notifications
)

ecm_find_qmlmodule(org.kde.kirigami REQUIRED)
ecm_find_qmlmodule(org.kde.kirigamiaddons.settings REQUIRED)
ecm_find_qmlmodule(org.kde.kirigamiaddons.formcard REQUIRED)

set(BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(BUILD_TESTING OFF CACHE BOOL "" FORCE)

# Read Google OAuth credentials from .env or environment if present
if(EXISTS "${CMAKE_SOURCE_DIR}/.env")
    file(STRINGS "${CMAKE_SOURCE_DIR}/.env" ENV_LINES)
    foreach(line ${ENV_LINES})
        string(STRIP "${line}" line)
        if(line MATCHES "^CLIENT_ID[ \t]*=[ \t]*['\"]?([^'\"]+)['\"]?")
            set(ENV_CLIENT_ID "${CMAKE_MATCH_1}")
        elseif(line MATCHES "^CLIENT_SECRET[ \t]*=[ \t]*['\"]?([^'\"]+)['\"]?")
            set(ENV_CLIENT_SECRET "${CMAKE_MATCH_1}")
        endif()
    endforeach()
endif()

if(NOT GOOGLE_CLIENT_ID AND ENV_CLIENT_ID)
    set(GOOGLE_CLIENT_ID "${ENV_CLIENT_ID}")
elseif(NOT GOOGLE_CLIENT_ID AND DEFINED ENV{CLIENT_ID})
    set(GOOGLE_CLIENT_ID "$ENV{CLIENT_ID}")
endif()

if(NOT GOOGLE_CLIENT_SECRET AND ENV_CLIENT_SECRET)
    set(GOOGLE_CLIENT_SECRET "${ENV_CLIENT_SECRET}")
elseif(NOT GOOGLE_CLIENT_SECRET AND DEFINED ENV{CLIENT_SECRET})
    set(GOOGLE_CLIENT_SECRET "$ENV{CLIENT_SECRET}")
endif()

if(GOOGLE_CLIENT_ID)
    add_compile_definitions(CHECKD_GOOGLE_CLIENT_ID="${GOOGLE_CLIENT_ID}")
endif()
if(GOOGLE_CLIENT_SECRET)
    add_compile_definitions(CHECKD_GOOGLE_CLIENT_SECRET="${GOOGLE_CLIENT_SECRET}")
endif()

add_subdirectory(3rdparty/AsyncSql)

add_subdirectory(src)
add_subdirectory(tests)

install(PROGRAMS com.kumar.checkd.desktop checkd.desktop DESTINATION ${KDE_INSTALL_APPDIR})
install(FILES com.kumar.checkd.metainfo.xml DESTINATION ${KDE_INSTALL_METAINFODIR})
install(FILES checkd.notifyrc DESTINATION ${KDE_INSTALL_KNOTIFYRCDIR})
install(FILES icons/com.kumar.checkd.svg DESTINATION ${KDE_INSTALL_ICONDIR}/hicolor/scalable/apps)
install(FILES icons/com.kumar.checkd.svg DESTINATION ${KDE_INSTALL_ICONDIR}/hicolor/scalable/apps RENAME checkd.svg)
foreach(size 16 22 24 32 48 64 128 256)
    install(FILES icons/${size}x${size}/apps/com.kumar.checkd.png DESTINATION ${KDE_INSTALL_ICONDIR}/hicolor/${size}x${size}/apps)
    install(FILES icons/${size}x${size}/apps/com.kumar.checkd.png DESTINATION ${KDE_INSTALL_ICONDIR}/hicolor/${size}x${size}/apps RENAME checkd.png)
endforeach()

feature_summary(WHAT ALL INCLUDE_QUIET_PACKAGES FATAL_ON_MISSING_REQUIRED_PACKAGES)
