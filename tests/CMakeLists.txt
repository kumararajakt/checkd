add_executable(tst_sidebarlistmodel
    tst_sidebarlistmodel.cpp
    ../src/databasemanager.cpp
    ../src/sidebarlistmodel.cpp
)

target_include_directories(tst_sidebarlistmodel PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../src
)

target_compile_options(tst_sidebarlistmodel PRIVATE -fexceptions)

target_link_libraries(tst_sidebarlistmodel PRIVATE
    Qt6::Core
    Qt6::Test
    Qt6::Sql
    Qt6::Qml
    asyncsql
)

add_test(NAME tst_sidebarlistmodel COMMAND tst_sidebarlistmodel)

add_executable(tst_tasklistmodel
    tst_tasklistmodel.cpp
    ../src/databasemanager.cpp
    ../src/sidebarlistmodel.cpp
    ../src/tasklistmodel.cpp
    ../src/notificationmanager.cpp
)

target_include_directories(tst_tasklistmodel PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../src
)

target_compile_options(tst_tasklistmodel PRIVATE -fexceptions)

target_link_libraries(tst_tasklistmodel PRIVATE
    Qt6::Core
    Qt6::Test
    Qt6::Sql
    Qt6::Qml
    KF6::I18n
    KF6::Notifications
    asyncsql
)

add_test(NAME tst_tasklistmodel COMMAND tst_tasklistmodel)

add_executable(tst_notificationmanager
    tst_notificationmanager.cpp
    ../src/databasemanager.cpp
    ../src/notificationmanager.cpp
)

target_include_directories(tst_notificationmanager PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../src
)

target_compile_options(tst_notificationmanager PRIVATE -fexceptions)

target_link_libraries(tst_notificationmanager PRIVATE
    Qt6::Core
    Qt6::Test
    Qt6::Sql
    Qt6::Gui
    KF6::I18n
    KF6::Notifications
    asyncsql
)

add_test(NAME tst_notificationmanager COMMAND tst_notificationmanager)

add_executable(tst_appsettings
    tst_appsettings.cpp
    ../src/appsettings.cpp
)

target_include_directories(tst_appsettings PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../src
)

target_compile_options(tst_appsettings PRIVATE -fexceptions)

target_compile_definitions(tst_appsettings PRIVATE
    SOURCE_DIR="${CMAKE_CURRENT_SOURCE_DIR}/.."
)

target_link_libraries(tst_appsettings PRIVATE
    Qt6::Core
    Qt6::Test
    Qt6::Quick
    Qt6::Qml
    KF6::Kirigami
)

add_test(NAME tst_appsettings COMMAND tst_appsettings)
