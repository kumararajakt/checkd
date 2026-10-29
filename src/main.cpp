#include <QApplication>
#include <QQmlApplicationEngine>
#include <QtQml>
#include <QUrl>
#include <QQuickStyle>
#include <KLocalizedContext>
#include <KLocalizedString>
#include <KIconTheme>

#include <QQuickWindow>
#include "appsettings.h"
#include "databasemanager.h"
#include "notificationmanager.h"
#include "sidebarlistmodel.h"
#include "systemtray.h"
#include "tasklistmodel.h"

int main(int argc, char *argv[])
{
    qSetMessagePattern(QStringLiteral("%{time hh:mm:ss.zzz} [%{type}] %{message}"));
    KLocalizedString::setApplicationDomain(QByteArrayLiteral("checkd"));
    KIconTheme::initTheme();
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("checkd"));
    QApplication::setDesktopFileName(QStringLiteral("com.kumar.checkd"));
    QApplication::setWindowIcon(QIcon::fromTheme(QStringLiteral("com.kumar.checkd"), QIcon::fromTheme(QStringLiteral("checkd"))));
    app.setQuitOnLastWindowClosed(false);

    DatabaseManager::instance().initialize();
    qmlRegisterType<SidebarListModel>("org.checkd", 1, 0, "SidebarListModel");
    qmlRegisterType<TaskListModel>("org.checkd", 1, 0, "TaskListModel");
    qmlRegisterSingletonInstance<AppSettings>("org.checkd", 1, 0, "AppSettings", &AppSettings::instance());

    QApplication::setStyle(QStringLiteral("breeze"));
    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }

    QQmlApplicationEngine engine;

    SystemTray systemTray(nullptr, &app);
    engine.rootContext()->setContextProperty(QStringLiteral("systemTray"), &systemTray);

    engine.rootContext()->setContextObject(new KLocalizedContext(&engine));
    engine.rootContext()->setContextProperty(QStringLiteral("appSettings"), &AppSettings::instance());
    engine.loadFromModule("org.checkd", "Main");

    if (engine.rootObjects().isEmpty()) {
        return -1;
    }

    QQuickWindow *mainWindow = qobject_cast<QQuickWindow*>(engine.rootObjects().first());
    systemTray.setAssociatedWindow(mainWindow);

    NotificationManager::instance().setAssociatedWindow(mainWindow);
    engine.rootContext()->setContextProperty(QStringLiteral("notificationManager"), &NotificationManager::instance());
    NotificationManager::instance().resetNotifications();
    NotificationManager::instance().checkNotifications();

    return app.exec();
}
