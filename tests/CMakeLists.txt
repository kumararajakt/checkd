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
