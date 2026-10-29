#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QSignalSpy>
#include <QStandardPaths>
#include <QTest>

#include "appsettings.h"

class TestAppSettings : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase();
    void testCloseToTray();
    void testAutostart();
    void testQmlGeneralSettingsPage();
    void cleanupTestCase();

private:
    QString m_autostartFile;
};

void TestAppSettings::initTestCase()
{
    QCoreApplication::setOrganizationName(QStringLiteral("kumar_test"));
    QCoreApplication::setApplicationName(QStringLiteral("checkd_test"));

    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    m_autostartFile = QDir(configDir).filePath(QStringLiteral("autostart/checkd.desktop"));
}

void TestAppSettings::testCloseToTray()
{
    AppSettings &settings = AppSettings::instance();
    QSignalSpy spy(&settings, &AppSettings::closeToTrayChanged);

    // Initial value should be true
    settings.setCloseToTray(true);
    QCOMPARE(settings.closeToTray(), true);

    // Toggle to false
    settings.setCloseToTray(false);
    QCOMPARE(settings.closeToTray(), false);
    QCOMPARE(spy.count(), 1);

    // Setting same value shouldn't emit signal
    settings.setCloseToTray(false);
    QCOMPARE(spy.count(), 1);

    // Restore to true
    settings.setCloseToTray(true);
    QCOMPARE(settings.closeToTray(), true);
    QCOMPARE(spy.count(), 2);
}

void TestAppSettings::testAutostart()
{
    AppSettings &settings = AppSettings::instance();
    QSignalSpy spy(&settings, &AppSettings::autostartChanged);

    // Turn off first
    settings.setAutostart(false);
    QCOMPARE(settings.autostart(), false);
    QCOMPARE(QFile::exists(m_autostartFile), false);

    spy.clear();

    // Turn on
    settings.setAutostart(true);
    QCOMPARE(settings.autostart(), true);
    QCOMPARE(QFile::exists(m_autostartFile), true);
    QCOMPARE(spy.count(), 1);

    // Turn off
    settings.setAutostart(false);
    QCOMPARE(settings.autostart(), false);
    QCOMPARE(QFile::exists(m_autostartFile), false);
    QCOMPARE(spy.count(), 2);
}

void TestAppSettings::testQmlGeneralSettingsPage()
{
    qmlRegisterSingletonInstance<AppSettings>("org.checkd", 1, 0, "AppSettings", &AppSettings::instance());
    QQmlEngine engine;
    QQmlComponent component(&engine, QUrl::fromLocalFile(QStringLiteral(SOURCE_DIR "/src/qml/GeneralSettingsPage.qml")));
    QVERIFY2(component.isReady(), qPrintable(component.errorString()));
    QObject *obj = component.create();
    QVERIFY(obj != nullptr);
    QCOMPARE(obj->property("title").toString(), QStringLiteral("General"));
    delete obj;
}

void TestAppSettings::cleanupTestCase()
{
    if (QFile::exists(m_autostartFile)) {
        QFile::remove(m_autostartFile);
    }
}

QTEST_MAIN(TestAppSettings)
#include "tst_appsettings.moc"
