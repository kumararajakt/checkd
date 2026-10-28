#include <QApplication>
#include <QQmlApplicationEngine>
#include <QtQml>
#include <QUrl>
#include <QQuickStyle>
#include <KLocalizedContext>
#include <KLocalizedString>
#include <KIconTheme>

#include "databasemanager.h"
#include "sidebarlistmodel.h"
#include "tasklistmodel.h"

int main(int argc, char *argv[])
{
    KIconTheme::initTheme();
    QApplication app(argc, argv);
    app.setOrganizationName(QStringLiteral("kumar"));
    app.setApplicationName(QStringLiteral("checkd"));

    DatabaseManager::instance().initialize();
    qmlRegisterType<SidebarListModel>("org.checkd", 1, 0, "SidebarListModel");
    qmlRegisterType<TaskListModel>("org.checkd", 1, 0, "TaskListModel");

    QApplication::setStyle(QStringLiteral("breeze"));
    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }

    QQmlApplicationEngine engine;

    engine.rootContext()->setContextObject(new KLocalizedContext(&engine));
    engine.loadFromModule("org.checkd", "Main");

    if (engine.rootObjects().isEmpty()) {
        return -1;
    }

    return app.exec();
}
