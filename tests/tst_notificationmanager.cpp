#include <QTest>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QJsonDocument>
#include <QJsonObject>
#include <KLocalizedString>
#include "databasemanager.h"
#include "notificationmanager.h"

class TestNotificationManager : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase() {
        KLocalizedString::setApplicationDomain(QByteArrayLiteral("checkd"));
        m_tempDir = std::make_unique<QTemporaryDir>();
        QVERIFY(m_tempDir->isValid());

        const QString dbPath = m_tempDir->filePath(QStringLiteral("test_notif.db"));
        QVERIFY(DatabaseManager::instance().initialize(dbPath));
    }

    void testComputeNextOccurrenceDaily() {
        QString cfg = QStringLiteral("{\"type\":\"day\",\"interval\":2}");
        QDate from(2026, 10, 20);
        QString next = NotificationManager::computeNextOccurrence(cfg, from);
        QCOMPARE(next, QStringLiteral("2026-10-22"));
    }

    void testComputeNextOccurrenceWeekly() {
        // Repeat on Mon (1) and Wed (3). From Tue 2026-10-20 (which is Tuesday, day 2):
        // Next should be Wed 2026-10-21.
        QString cfg = QStringLiteral("{\"type\":\"week\",\"interval\":1,\"days\":[1,3]}");
        QDate from(2026, 10, 20); // Tuesday
        QString next = NotificationManager::computeNextOccurrence(cfg, from);
        QCOMPARE(next, QStringLiteral("2026-10-21"));
    }

    void testComputeNextOccurrenceMonthly() {
        QString cfg = QStringLiteral("{\"type\":\"month\",\"interval\":1}");
        QDate from(2026, 10, 15);
        QString next = NotificationManager::computeNextOccurrence(cfg, from);
        QCOMPARE(next, QStringLiteral("2026-11-15"));
    }

    void testDueAndRepeatNotificationTrigger() {
        const QString dbPath = DatabaseManager::instance().databasePath();
        const QString conn = QStringLiteral("test_notif_insert");
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            // Insert list
            q.exec(QStringLiteral("INSERT INTO lists (id, name, color, position, updated_at) VALUES ('list1', 'Work', '#ff0000', 1, 1000)"));

            // Insert overdue task
            q.prepare(QStringLiteral("INSERT INTO tasks (id, list_id, title, description, due_date, due_time, position, updated_at, completed) "
                                     "VALUES ('task-due', 'list1', 'Submit Report', 'urgent', '2026-10-01', '10:00', 1, 1000, 0)"));
            QVERIFY(q.exec());

            // Insert repeating task due today
            const QString todayStr = QDate::currentDate().toString(QStringLiteral("yyyy-MM-dd"));
            q.prepare(QStringLiteral("INSERT INTO tasks (id, list_id, title, description, repeat_json, next_occurrence, position, updated_at, completed) "
                                     "VALUES ('task-rep', 'list1', 'Daily Standup', '', '{\"type\":\"day\",\"interval\":1}', :today, 2, 1000, 0)"));
            q.bindValue(QStringLiteral(":today"), todayStr);
            QVERIFY(q.exec());

            db.close();
        }
        QSqlDatabase::removeDatabase(conn);

        // Run checkNotifications
        NotificationManager::instance().checkNotifications();

        // Verify notified_key was updated for both tasks
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            q.exec(QStringLiteral("SELECT notified_key FROM tasks WHERE id = 'task-due'"));
            QVERIFY(q.next());
            QCOMPARE(q.value(0).toString(), QStringLiteral("due:2026-10-01T10:00"));

            q.exec(QStringLiteral("SELECT notified_key FROM tasks WHERE id = 'task-rep'"));
            QVERIFY(q.next());
            QVERIFY(q.value(0).toString().startsWith(QStringLiteral("occ:")));

            db.close();
        }
        QSqlDatabase::removeDatabase(conn);
    }

    void testMarkTaskCompletedFromNotification() {
        QSignalSpy spy(&NotificationManager::instance(), &NotificationManager::taskStateChanged);

        // Complete regular task
        NotificationManager::instance().markTaskCompleted(QStringLiteral("task-due"));
        QCOMPARE(spy.count(), 1);

        // Complete repeating task -> should advance next_occurrence, reset notified_key, and remain completed = 0
        NotificationManager::instance().markTaskCompleted(QStringLiteral("task-rep"));
        QCOMPARE(spy.count(), 2);

        const QString dbPath = DatabaseManager::instance().databasePath();
        const QString conn = QStringLiteral("test_notif_verify");
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            q.exec(QStringLiteral("SELECT completed FROM tasks WHERE id = 'task-due'"));
            QVERIFY(q.next());
            QCOMPARE(q.value(0).toInt(), 1);

            q.exec(QStringLiteral("SELECT completed, next_occurrence, notified_key, occurrence_index FROM tasks WHERE id = 'task-rep'"));
            QVERIFY(q.next());
            QCOMPARE(q.value(0).toInt(), 0); // Still 0 because repeating
            QVERIFY(q.value(1).toString() > QDate::currentDate().toString(QStringLiteral("yyyy-MM-dd")));
            QVERIFY(q.value(2).isNull() || q.value(2).toString().isEmpty()); // notified_key cleared
            QCOMPARE(q.value(3).toInt(), 1); // occurrence_index incremented

            db.close();
        }
        QSqlDatabase::removeDatabase(conn);
    }

    void testResetNotifications() {
        const QString dbPath = DatabaseManager::instance().databasePath();
        const QString conn = QStringLiteral("test_reset_notif");
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            q.exec(QStringLiteral("UPDATE tasks SET notified_key = 'due:already_notified' WHERE id = 'task-rep'"));
            db.close();
        }
        QSqlDatabase::removeDatabase(conn);

        NotificationManager::instance().resetNotifications();

        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            q.exec(QStringLiteral("SELECT notified_key FROM tasks WHERE id = 'task-rep'"));
            QVERIFY(q.next());
            QVERIFY(q.value(0).isNull() || q.value(0).toString().isEmpty());
            db.close();
        }
        QSqlDatabase::removeDatabase(conn);
    }

    void testRepeatingTaskWithDueDateDoesNotDuplicate() {
        const QString dbPath = DatabaseManager::instance().databasePath();
        const QString conn = QStringLiteral("test_notif_repeat_due");
        const QString todayStr = QDate::currentDate().toString(QStringLiteral("yyyy-MM-dd"));
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            // Insert task with BOTH dueDate and repeatJson (as created when adding repeat in UI)
            q.prepare(QStringLiteral("INSERT INTO tasks (id, list_id, title, description, due_date, due_time, repeat_json, position, updated_at, completed) "
                                     "VALUES ('task-rep-both', 'list1', 'Morning Standup', '', :today, '08:00', :repeatJson, 3, 1000, 0)"));
            q.bindValue(QStringLiteral(":today"), todayStr);
            const QString repeatJson = QStringLiteral("{\"type\":\"day\",\"interval\":1,\"startDate\":\"") + todayStr + QStringLiteral("\",\"time\":\"08:00\"}");
            q.bindValue(QStringLiteral(":repeatJson"), repeatJson);
            QVERIFY(q.exec());
            db.close();
        }
        QSqlDatabase::removeDatabase(conn);

        // First check
        NotificationManager::instance().checkNotifications();

        QString keyAfterFirstCheck;
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            q.exec(QStringLiteral("SELECT notified_key FROM tasks WHERE id = 'task-rep-both'"));
            QVERIFY(q.next());
            keyAfterFirstCheck = q.value(0).toString();
            // Should be occ:todayT08:00, NOT ping-ponged with due:todayT08:00
            QCOMPARE(keyAfterFirstCheck, QStringLiteral("occ:") + todayStr + QStringLiteral("T08:00"));
            db.close();
        }
        QSqlDatabase::removeDatabase(conn);

        // Second check (simulating next timer tick)
        NotificationManager::instance().checkNotifications();

        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), conn);
            db.setDatabaseName(dbPath);
            QVERIFY(db.open());

            QSqlQuery q(db);
            q.exec(QStringLiteral("SELECT notified_key FROM tasks WHERE id = 'task-rep-both'"));
            QVERIFY(q.next());
            // Key should remain unchanged and stable
            QCOMPARE(q.value(0).toString(), keyAfterFirstCheck);
            db.close();
        }
        QSqlDatabase::removeDatabase(conn);
    }

    void cleanupTestCase() {
        QTest::qWait(50);
        QCoreApplication::processEvents();
        m_tempDir.reset();
    }

private:
    std::unique_ptr<QTemporaryDir> m_tempDir;
};

QTEST_MAIN(TestNotificationManager)
#include "tst_notificationmanager.moc"
