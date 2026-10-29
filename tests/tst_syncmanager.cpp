#include <QTest>
#include <QTemporaryDir>
#include <QSignalSpy>
#include <QSqlDatabase>
#include <QSqlQuery>
#include "databasemanager.h"
#include "googleauth.h"
#include "syncmanager.h"

class TestSyncManager : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase() {
        m_tempDir = std::make_unique<QTemporaryDir>();
        QVERIFY(m_tempDir->isValid());

        const QString dbPath = m_tempDir->filePath(QStringLiteral("test_sync.db"));
        QVERIFY(DatabaseManager::instance().initialize(dbPath));
    }

    void testDefaultCredentials() {
#if defined(CHECKD_GOOGLE_CLIENT_ID) && defined(CHECKD_GOOGLE_CLIENT_SECRET)
        QCOMPARE(GoogleAuth::defaultClientId(), QStringLiteral(CHECKD_GOOGLE_CLIENT_ID));
        QCOMPARE(GoogleAuth::defaultClientSecret(), QStringLiteral(CHECKD_GOOGLE_CLIENT_SECRET));
        QVERIFY(GoogleAuth::isAvailable());
        QVERIFY(SyncManager::instance().isAvailable());
#else
        if (qEnvironmentVariableIsEmpty("CLIENT_ID") && qEnvironmentVariableIsEmpty("GOOGLE_CLIENT_ID")) {
            QVERIFY(GoogleAuth::defaultClientId().isEmpty());
            QVERIFY(!GoogleAuth::isAvailable());
            QVERIFY(!SyncManager::instance().isAvailable());
        }
#endif
    }

    void testInitialStateNotSignedIn() {
        GoogleAuth::instance().signOut();
        QVERIFY(!GoogleAuth::instance().isSignedIn());
        QVERIFY(!SyncManager::instance().isSignedIn());
        QVERIFY(!SyncManager::instance().isSyncing());
    }

    void testSyncWhenNotSignedIn() {
        QSignalSpy errorSpy(&SyncManager::instance(), &SyncManager::lastErrorChanged);
        QSignalSpy finishSpy(&SyncManager::instance(), &SyncManager::syncFinished);

        SyncManager::instance().sync();

        QVERIFY(errorSpy.count() >= 1);
        QCOMPARE(finishSpy.count(), 1);
        QCOMPARE(finishSpy.first().at(0).toBool(), false);
    }

    void cleanupTestCase() {
        m_tempDir.reset();
    }

private:
    std::unique_ptr<QTemporaryDir> m_tempDir;
};

QTEST_MAIN(TestSyncManager)
#include "tst_syncmanager.moc"
