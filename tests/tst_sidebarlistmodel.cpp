#include <QTest>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QSqlQuery>

#include "databasemanager.h"
#include "sidebarlistmodel.h"

class TestSidebarListModel : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase();
    void cleanupTestCase();

    void testInitialLoad();
    void testAddList();
    void testRenameList();
    void testSetListColor();
    void testGetMethod();
    void testDeleteList();

private:
    QTemporaryDir m_tempDir;
    QString m_dbPath;
};

void TestSidebarListModel::initTestCase()
{
    QVERIFY(m_tempDir.isValid());
    m_dbPath = m_tempDir.filePath(QStringLiteral("test_checkd.db"));

    QVERIFY(DatabaseManager::instance().initialize(m_dbPath));
    QCOMPARE(DatabaseManager::instance().databasePath(), m_dbPath);
}

void TestSidebarListModel::cleanupTestCase()
{
    QTest::qWait(200);
}

void TestSidebarListModel::testInitialLoad()
{
    SidebarListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy.wait(3000));

    // Initially table is empty
    QCOMPARE(model.rowCount(), 0);
    QCOMPARE(model.property("count").toInt(), 0);
}

void TestSidebarListModel::testAddList()
{
    SidebarListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy.wait(3000));

    QSignalSpy countSpy(&model, &SidebarListModel::countChanged);
    model.addList(QStringLiteral("Work Tasks"), QStringLiteral("#e74c3c"));

    QCOMPARE(model.rowCount(), 1);
    QCOMPARE(countSpy.count(), 1);

    // Verify roles via data()
    QModelIndex idx = model.index(0, 0);
    QCOMPARE(model.data(idx, SidebarListModel::NameRole).toString(), QStringLiteral("Work Tasks"));
    QCOMPARE(model.data(idx, SidebarListModel::LabelRole).toString(), QStringLiteral("Work Tasks"));
    QCOMPARE(model.data(idx, SidebarListModel::ColorRole).toString(), QStringLiteral("#e74c3c"));
    QCOMPARE(model.data(idx, SidebarListModel::ModelColorRole).toString(), QStringLiteral("#e74c3c"));
    QVERIFY(!model.data(idx, SidebarListModel::IdRole).toString().isEmpty());

    // Wait a brief moment to ensure async persist completes
    QTest::qWait(100);

    // Verify persistence with a second model
    SidebarListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.rowCount(), 1);
    QModelIndex idx2 = model2.index(0, 0);
    QCOMPARE(model2.data(idx2, SidebarListModel::NameRole).toString(), QStringLiteral("Work Tasks"));
}

void TestSidebarListModel::testRenameList()
{
    SidebarListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy.wait(3000));
    QCOMPARE(model.rowCount(), 1);

    // Test renaming via setProperty (as SidebarListDelegate does)
    QVERIFY(model.setProperty(0, QStringLiteral("label"), QStringLiteral("Urgent Work")));
    QModelIndex idx = model.index(0, 0);
    QCOMPARE(model.data(idx, SidebarListModel::NameRole).toString(), QStringLiteral("Urgent Work"));

    QTest::qWait(100);

    SidebarListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.data(model2.index(0, 0), SidebarListModel::NameRole).toString(), QStringLiteral("Urgent Work"));
}

void TestSidebarListModel::testSetListColor()
{
    SidebarListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy.wait(3000));
    QCOMPARE(model.rowCount(), 1);

    // Test color change via setProperty (as ColorPickerPopup does)
    QVERIFY(model.setProperty(0, QStringLiteral("modelColor"), QStringLiteral("#2ecc71")));
    QModelIndex idx = model.index(0, 0);
    QCOMPARE(model.data(idx, SidebarListModel::ColorRole).toString(), QStringLiteral("#2ecc71"));

    QTest::qWait(100);

    SidebarListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.data(model2.index(0, 0), SidebarListModel::ColorRole).toString(), QStringLiteral("#2ecc71"));
}

void TestSidebarListModel::testGetMethod()
{
    SidebarListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy.wait(3000));

    QVariantMap item = model.get(0);
    QCOMPARE(item.value(QStringLiteral("name")).toString(), QStringLiteral("Urgent Work"));
    QCOMPARE(item.value(QStringLiteral("label")).toString(), QStringLiteral("Urgent Work"));
    QCOMPARE(item.value(QStringLiteral("color")).toString(), QStringLiteral("#2ecc71"));
    QCOMPARE(item.value(QStringLiteral("modelColor")).toString(), QStringLiteral("#2ecc71"));
    QVERIFY(!item.value(QStringLiteral("id")).toString().isEmpty());
}

void TestSidebarListModel::testDeleteList()
{
    SidebarListModel model;
    QSignalSpy selectSpy(&model, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy.wait(3000));
    QCOMPARE(model.rowCount(), 1);

    // Test remove(0) (as SidebarListDelegate delete button does)
    QSignalSpy countSpy(&model, &SidebarListModel::countChanged);
    QVERIFY(model.remove(0));
    QCOMPARE(model.rowCount(), 0);
    QCOMPARE(countSpy.count(), 1);

    QTest::qWait(100);

    SidebarListModel model2;
    QSignalSpy selectSpy2(&model2, &AsyncSql::AsyncSqlBaseModel::selected);
    QVERIFY(selectSpy2.wait(3000));
    QCOMPARE(model2.rowCount(), 0);
}

QTEST_MAIN(TestSidebarListModel)
#include "tst_sidebarlistmodel.moc"
