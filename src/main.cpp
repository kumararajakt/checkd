#include <QApplication>
#include <QQmlApplicationEngine>
#include <QtQml>
#include <QUrl>
#include <QQuickStyle>
#include <KLocalizedContext>
#include <KLocalizedString>
#include <KIconTheme>

#include <QQuickWindow>
#include "databasemanager.h"
#include "sidebarlistmodel.h"
#include "systemtray.h"
#include "tasklistmodel.h"

int main(int argc, char *argv[])
{
    qSetMessagePattern(QStringLiteral("%{time hh:mm:ss.zzz} [%{type}] %{message}"));
    KLocalizedString::setApplicationDomain(QByteArrayLiteral("checkd"));
    KIconTheme::initTheme();
    QApplication app(argc, argv);
    app.setOrganizationName(QStringLiteral("kumar"));
    app.setApplicationName(QStringLiteral("checkd"));
    app.setQuitOnLastWindowClosed(false);

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

    QQuickWindow *mainWindow = qobject_cast<QQuickWindow*>(engine.rootObjects().first());
    SystemTray systemTray(mainWindow, &app);
    engine.rootContext()->setContextProperty(QStringLiteral("systemTray"), &systemTray);

    return app.exec();
}
