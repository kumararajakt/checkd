#include <QTest>
#include <QSignalSpy>
#include <QTemporaryDir>

#include "databasemanager.h"
#include "sidebarlistmodel.h"
#include "tasklistmodel.h"
#include "taskfilterproxymodel.h"

class TestTaskListModel : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase();
    void cleanupTestCase();

    void testInitialEmpty();
    void testAddTask();
    void testSetCompleted();
    void testUpdateDetails();
    void testGetMethod();
    void testDeleteTask();
    void testTaskFilterProxyModel();

private:
    QTemporaryDir m_tempDir;
    QString m_dbPath;
    QString m_listId;
};

void TestTaskListModel::initTestCase()
{
    QVERIFY(m_tempDir.isValid());
    m_dbPath = m_tempDir.filePath(QStringLiteral("test_checkd_tasks.db"));

    QVERIFY(DatabaseManager::instance().initialize(m_dbPath));
    QCOMPARE(DatabaseManager::instance().databasePath(), m_dbPath);

    // Create a list to associate tasks with
    SidebarListModel listModel;
    QSignalSpy listSpy(&listModel, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(listSpy.wait(3000));

    listModel.addList(QStringLiteral("Task Test List"), QStringLiteral("#3b82f6"));
    QTest::qWait(100);

    QVariantMap list = listModel.get(0);
    m_listId = list.value(QStringLiteral("id")).toString();
    QVERIFY(!m_listId.isEmpty());
}

void TestTaskListModel::cleanupTestCase()
{
    QTest::qWait(200);
}

void TestTaskListModel::testInitialEmpty()
{
    TaskListModel model;
    QCOMPARE(model.rowCount(), 0);

    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    model.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));

    QCOMPARE(model.rowCount(), 0);
    QCOMPARE(model.property("count").toInt(), 0);
}

void TestTaskListModel::testAddTask()
{
    TaskListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    model.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));

    QSignalSpy countSpy(&model, &TaskListModel::countChanged);
    model.addTask(QStringLiteral("First Task"), QStringLiteral("My description"), QStringLiteral("2026-11-01"), QStringLiteral("09:00"));

    QCOMPARE(model.rowCount(), 1);
    QCOMPARE(countSpy.count(), 1);

    QModelIndex idx = model.index(0, 0);
    QCOMPARE(model.data(idx, TaskListModel::TitleRole).toString(), QStringLiteral("First Task"));
    QCOMPARE(model.data(idx, TaskListModel::DescriptionRole).toString(), QStringLiteral("My description"));
    QCOMPARE(model.data(idx, TaskListModel::DueDateRole).toString(), QStringLiteral("2026-11-01"));
    QCOMPARE(model.data(idx, TaskListModel::DueTimeRole).toString(), QStringLiteral("09:00"));
    QCOMPARE(model.data(idx, TaskListModel::IsCompletedRole).toBool(), false);
    QVERIFY(!model.data(idx, TaskListModel::IdRole).toString().isEmpty());

    QTest::qWait(100);

    // Verify persistence with a fresh model
    TaskListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    model2.setListId(m_listId);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.rowCount(), 1);
    QModelIndex idx2 = model2.index(0, 0);
    QCOMPARE(model2.data(idx2, TaskListModel::TitleRole).toString(), QStringLiteral("First Task"));
    QCOMPARE(model2.data(idx2, TaskListModel::DescriptionRole).toString(), QStringLiteral("My description"));
}

void TestTaskListModel::testSetCompleted()
{
    TaskListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    model.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));
    QCOMPARE(model.rowCount(), 1);

    // Toggle completed via setProperty
    QVERIFY(model.setProperty(0, QStringLiteral("isCompleted"), true));
    QModelIndex idx = model.index(0, 0);
    QCOMPARE(model.data(idx, TaskListModel::IsCompletedRole).toBool(), true);

    QTest::qWait(100);

    TaskListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    model2.setListId(m_listId);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.data(model2.index(0, 0), TaskListModel::IsCompletedRole).toBool(), true);
}

void TestTaskListModel::testUpdateDetails()
{
    TaskListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    model.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));

    // Update title, description, due date, due time
    QVERIFY(model.setProperty(0, QStringLiteral("title"), QStringLiteral("Updated Task Title")));
    QVERIFY(model.setProperty(0, QStringLiteral("description"), QStringLiteral("Updated Desc")));
    QVERIFY(model.setProperty(0, QStringLiteral("dueDate"), QStringLiteral("2026-12-25")));
    QVERIFY(model.setProperty(0, QStringLiteral("dueTime"), QStringLiteral("18:30")));

    QModelIndex idx = model.index(0, 0);
    QCOMPARE(model.data(idx, TaskListModel::TitleRole).toString(), QStringLiteral("Updated Task Title"));
    QCOMPARE(model.data(idx, TaskListModel::DescriptionRole).toString(), QStringLiteral("Updated Desc"));
    QCOMPARE(model.data(idx, TaskListModel::DueDateRole).toString(), QStringLiteral("2026-12-25"));
    QCOMPARE(model.data(idx, TaskListModel::DueTimeRole).toString(), QStringLiteral("18:30"));

    QTest::qWait(100);

    TaskListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    model2.setListId(m_listId);
    QVERIFY(selectSpy2.wait(3000));
    QModelIndex idx2 = model2.index(0, 0);
    QCOMPARE(model2.data(idx2, TaskListModel::TitleRole).toString(), QStringLiteral("Updated Task Title"));
    QCOMPARE(model2.data(idx2, TaskListModel::DueDateRole).toString(), QStringLiteral("2026-12-25"));
}

void TestTaskListModel::testGetMethod()
{
    TaskListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    model.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));

    QVariantMap task = model.get(0);
    QCOMPARE(task.value(QStringLiteral("title")).toString(), QStringLiteral("Updated Task Title"));
    QCOMPARE(task.value(QStringLiteral("description")).toString(), QStringLiteral("Updated Desc"));
    QCOMPARE(task.value(QStringLiteral("dueDate")).toString(), QStringLiteral("2026-12-25"));
    QCOMPARE(task.value(QStringLiteral("dueTime")).toString(), QStringLiteral("18:30"));
    QCOMPARE(task.value(QStringLiteral("isCompleted")).toBool(), true);
    QCOMPARE(task.value(QStringLiteral("listId")).toString(), m_listId);
    QVERIFY(!task.value(QStringLiteral("id")).toString().isEmpty());
}

void TestTaskListModel::testDeleteTask()
{
    TaskListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    model.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));
    QCOMPARE(model.rowCount(), 1);

    QSignalSpy countSpy(&model, &TaskListModel::countChanged);
    QVERIFY(model.remove(0));
    QCOMPARE(model.rowCount(), 0);
    QCOMPARE(countSpy.count(), 1);

    QTest::qWait(100);

    TaskListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    model2.setListId(m_listId);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.rowCount(), 0);
}

void TestTaskListModel::testTaskFilterProxyModel()
{
    TaskListModel taskModel;
    QSignalSpy selectSpy(&taskModel, &AsyncSql::AsyncSqlBaseModel::selected);
    taskModel.setListId(m_listId);
    QVERIFY(selectSpy.wait(3000));

    taskModel.addTask(QStringLiteral("Active Task 1"));
    taskModel.addTask(QStringLiteral("Task to complete"));

    QCOMPARE(taskModel.rowCount(), 2);

    TaskFilterProxyModel activeProxy;
    activeProxy.setSourceTaskListModel(&taskModel);
    activeProxy.setFilterCompleted(false);

    TaskFilterProxyModel completedProxy;
    completedProxy.setSourceTaskListModel(&taskModel);
    completedProxy.setFilterCompleted(true);

    QCOMPARE(activeProxy.rowCount(), 2);
    QCOMPARE(completedProxy.rowCount(), 0);

    // Complete the second task
    activeProxy.setCompleted(1, true);

    QCOMPARE(activeProxy.rowCount(), 1);
    QCOMPARE(completedProxy.rowCount(), 1);
    QCOMPARE(activeProxy.data(activeProxy.index(0, 0), TaskListModel::TitleRole).toString(), QStringLiteral("Active Task 1"));
    QCOMPARE(completedProxy.data(completedProxy.index(0, 0), TaskListModel::TitleRole).toString(), QStringLiteral("Task to complete"));

    // Uncomplete from completed proxy
    completedProxy.setCompleted(0, false);
    QCOMPARE(activeProxy.rowCount(), 2);
    QCOMPARE(completedProxy.rowCount(), 0);

    // Clean up
    taskModel.remove(0);
    taskModel.remove(0);
}

QTEST_MAIN(TestTaskListModel)
#include "tst_tasklistmodel.moc"
