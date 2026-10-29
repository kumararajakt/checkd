#include "syncmanager.h"
#include "databasemanager.h"
#include "googleauth.h"

#include <QDebug>
#include <QDateTime>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QUuid>

static const QString s_syncDbConn = QStringLiteral("sync_manager_conn");

SyncManager &SyncManager::instance()
{
    static SyncManager s_instance;
    return s_instance;
}

SyncManager::SyncManager(QObject *parent)
    : QObject(parent)
{
    connect(&GoogleAuth::instance(), &GoogleAuth::signedInChanged, this, &SyncManager::signedInChanged);
    connect(&GoogleAuth::instance(), &GoogleAuth::accountEmailChanged, this, &SyncManager::accountEmailChanged);
    connect(&GoogleAuth::instance(), &GoogleAuth::authSuccess, this, [this]() {
        sync();
    });
    connect(&GoogleAuth::instance(), &GoogleAuth::authError, this, [this](const QString &err) {
        m_lastError = err;
        Q_EMIT lastErrorChanged();
    });

    loadSyncState();
}

bool SyncManager::isAvailable() const
{
    return GoogleAuth::isAvailable();
}

bool SyncManager::isSignedIn() const
{
    return GoogleAuth::instance().isSignedIn();
}

QString SyncManager::accountEmail() const
{
    return GoogleAuth::instance().accountEmail();
}

bool SyncManager::isSyncing() const
{
    return m_syncing;
}

QString SyncManager::lastSyncTime() const
{
    return m_lastSyncTime;
}

QString SyncManager::lastError() const
{
    return m_lastError;
}

void SyncManager::loadSyncState()
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    if (dbPath.isEmpty()) {
        return;
    }

    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            q.prepare(QStringLiteral("SELECT value FROM sync_state WHERE key = 'last_synced_at'"));
            if (q.exec() && q.next()) {
                const qint64 ts = q.value(0).toLongLong();
                if (ts > 0) {
                    m_lastSyncTime = QDateTime::fromMSecsSinceEpoch(ts).toString(QStringLiteral("MMM d, yyyy · hh:mm AP"));
                }
            }
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(s_syncDbConn);
}

void SyncManager::saveLastSyncTime(qint64 timestamp)
{
    m_lastSyncTime = QDateTime::fromMSecsSinceEpoch(timestamp).toString(QStringLiteral("MMM d, yyyy · hh:mm AP"));
    Q_EMIT lastSyncTimeChanged();

    const QString dbPath = DatabaseManager::instance().databasePath();
    if (dbPath.isEmpty()) {
        return;
    }

    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            q.prepare(QStringLiteral("INSERT OR REPLACE INTO sync_state (key, value) VALUES ('last_synced_at', :val)"));
            q.bindValue(QStringLiteral(":val"), QString::number(timestamp));
            q.exec();
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(s_syncDbConn);
}

void SyncManager::signIn()
{
    if (!isAvailable()) {
        m_lastError = tr("Google Sync is not configured.");
        Q_EMIT lastErrorChanged();
        return;
    }
    m_lastError.clear();
    Q_EMIT lastErrorChanged();
    GoogleAuth::instance().startSignIn();
}

void SyncManager::signOut()
{
    GoogleAuth::instance().signOut();
    m_lastSyncTime.clear();
    Q_EMIT lastSyncTimeChanged();
    m_lastError.clear();
    Q_EMIT lastErrorChanged();
}

void SyncManager::sync()
{
    if (m_syncing) {
        return;
    }
    if (!isAvailable()) {
        m_lastError = tr("Google Sync is not configured.");
        Q_EMIT lastErrorChanged();
        Q_EMIT syncFinished(false, m_lastError);
        return;
    }
    if (!isSignedIn()) {
        m_lastError = tr("Not signed in.");
        Q_EMIT lastErrorChanged();
        Q_EMIT syncFinished(false, m_lastError);
        return;
    }

    m_syncing = true;
    m_lastError.clear();
    Q_EMIT syncingChanged();
    Q_EMIT lastErrorChanged();

    runSyncWorkflow();
}

void SyncManager::runSyncWorkflow()
{
    // Step 1: Pull remote calendars
    m_calendarService.listCalendars([this](bool ok, const QList<RemoteCalendar> &remoteCals) {
        if (!ok) {
            m_syncing = false;
            m_lastError = tr("Failed to fetch calendars from Google.");
            Q_EMIT syncingChanged();
            Q_EMIT lastErrorChanged();
            Q_EMIT syncFinished(false, m_lastError);
            return;
        }

        const QString dbPath = DatabaseManager::instance().databasePath();
        const qint64 now = QDateTime::currentMSecsSinceEpoch();

        // Process remote calendars and map to local lists
        QMap<QString, QString> remoteCalToLocalListId;
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
            db.setDatabaseName(dbPath);
            if (db.open()) {
                for (const RemoteCalendar &rc : remoteCals) {
                    QString localId;
                    // Check if mapped in sync_links
                    QSqlQuery q(db);
                    q.prepare(QStringLiteral("SELECT entity_id FROM sync_links WHERE entity_type = 'list' AND remote_id = :rId"));
                    q.bindValue(QStringLiteral(":rId"), rc.id);
                    if (q.exec() && q.next()) {
                        localId = q.value(0).toString();
                        // Update local list properties if not dirty
                        QSqlQuery checkDirty(db);
                        checkDirty.prepare(QStringLiteral("SELECT dirty FROM sync_links WHERE entity_type = 'list' AND entity_id = :id"));
                        checkDirty.bindValue(QStringLiteral(":id"), localId);
                        if (checkDirty.exec() && checkDirty.next() && checkDirty.value(0).toInt() == 0) {
                            QSqlQuery u(db);
                            u.prepare(QStringLiteral("UPDATE lists SET name = :n, color = :c, position = :p, updated_at = :now WHERE id = :id"));
                            u.bindValue(QStringLiteral(":n"), rc.title);
                            u.bindValue(QStringLiteral(":c"), rc.color);
                            u.bindValue(QStringLiteral(":p"), rc.index);
                            u.bindValue(QStringLiteral(":now"), now);
                            u.bindValue(QStringLiteral(":id"), localId);
                            u.exec();
                        }
                    } else {
                        // Check if a local list with same title exists
                        QSqlQuery match(db);
                        match.prepare(QStringLiteral("SELECT id FROM lists WHERE name = :name LIMIT 1"));
                        match.bindValue(QStringLiteral(":name"), rc.title);
                        if (match.exec() && match.next()) {
                            localId = match.value(0).toString();
                        } else {
                            // Create new local list
                            localId = QUuid::createUuid().toString(QUuid::WithoutBraces);
                            QSqlQuery ins(db);
                            ins.prepare(QStringLiteral("INSERT INTO lists (id, name, color, position, updated_at) VALUES (:id, :n, :c, :p, :now)"));
                            ins.bindValue(QStringLiteral(":id"), localId);
                            ins.bindValue(QStringLiteral(":n"), rc.title);
                            ins.bindValue(QStringLiteral(":c"), rc.color);
                            ins.bindValue(QStringLiteral(":p"), rc.index);
                            ins.bindValue(QStringLiteral(":now"), now);
                            ins.exec();
                        }

                        // Link in sync_links
                        QSqlQuery link(db);
                        link.prepare(QStringLiteral("INSERT OR REPLACE INTO sync_links (entity_type, entity_id, remote_id, dirty, deleted, updated_at, synced_at) "
                                                    "VALUES ('list', :id, :rId, 0, 0, :now, :now)"));
                        link.bindValue(QStringLiteral(":id"), localId);
                        link.bindValue(QStringLiteral(":rId"), rc.id);
                        link.bindValue(QStringLiteral(":now"), now);
                        link.exec();
                    }

                    remoteCalToLocalListId.insert(rc.id, localId);
                }
                db.close();
            }
        }
        QSqlDatabase::removeDatabase(s_syncDbConn);

        // Step 2: For each remote calendar, fetch events
        auto fetchEventsRecursive = std::make_shared<std::function<void(int)>>();
        *fetchEventsRecursive = [this, remoteCals, remoteCalToLocalListId, dbPath, now, fetchEventsRecursive](int index) {
            if (index >= remoteCals.size()) {
                // Done fetching all remote events! Proceed to Step 3: Push local changes
                // Push local dirty/deleted lists and tasks
                // Let's call pushLocalChanges
                struct DirtyItem {
                    QString entityType;
                    QString entityId;
                    QString parentId;
                    QString remoteId;
                    int dirty = 0;
                    int deleted = 0;
                };
                QList<DirtyItem> dirtyItems;

                {
                    QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                    db.setDatabaseName(dbPath);
                    if (db.open()) {
                        QSqlQuery q(db);
                        if (q.exec(QStringLiteral("SELECT entity_type, entity_id, parent_id, remote_id, dirty, deleted FROM sync_links WHERE dirty = 1 OR deleted = 1"))) {
                            while (q.next()) {
                                DirtyItem it;
                                it.entityType = q.value(0).toString();
                                it.entityId = q.value(1).toString();
                                it.parentId = q.value(2).toString();
                                it.remoteId = q.value(3).toString();
                                it.dirty = q.value(4).toInt();
                                it.deleted = q.value(5).toInt();
                                dirtyItems.append(it);
                            }
                        }
                        db.close();
                    }
                }
                QSqlDatabase::removeDatabase(s_syncDbConn);

                auto pushRecursive = std::make_shared<std::function<void(int)>>();
                *pushRecursive = [this, dirtyItems, dbPath, now, pushRecursive](int dIdx) {
                    if (dIdx >= dirtyItems.size()) {
                        // All synced!
                        saveLastSyncTime(now);
                        m_syncing = false;
                        Q_EMIT syncingChanged();
                        Q_EMIT syncFinished(true, tr("Sync completed successfully."));
                        Q_EMIT dataReloadRequested();
                        return;
                    }

                    const DirtyItem item = dirtyItems.at(dIdx);
                    if (item.entityType == QLatin1String("list")) {
                        if (item.deleted == 1) {
                            if (!item.remoteId.isEmpty()) {
                                m_calendarService.deleteCalendar(item.remoteId, [item, dbPath, pushRecursive, dIdx](bool) {
                                    // Delete local sync_link and list
                                    QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                    db.setDatabaseName(dbPath);
                                    if (db.open()) {
                                        QSqlQuery d(db);
                                        d.prepare(QStringLiteral("DELETE FROM sync_links WHERE entity_type = 'list' AND entity_id = :id"));
                                        d.bindValue(QStringLiteral(":id"), item.entityId);
                                        d.exec();
                                        d.prepare(QStringLiteral("DELETE FROM lists WHERE id = :id"));
                                        d.bindValue(QStringLiteral(":id"), item.entityId);
                                        d.exec();
                                        db.close();
                                    }
                                    QSqlDatabase::removeDatabase(s_syncDbConn);
                                    (*pushRecursive)(dIdx + 1);
                                });
                            } else {
                                (*pushRecursive)(dIdx + 1);
                            }
                        } else if (item.remoteId.isEmpty()) {
                            // Create remote calendar
                            QString title;
                            int pos = 0;
                            QString color;
                            {
                                QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                db.setDatabaseName(dbPath);
                                if (db.open()) {
                                    QSqlQuery q(db);
                                    q.prepare(QStringLiteral("SELECT name, position, color FROM lists WHERE id = :id"));
                                    q.bindValue(QStringLiteral(":id"), item.entityId);
                                    if (q.exec() && q.next()) {
                                        title = q.value(0).toString();
                                        pos = q.value(1).toInt();
                                        color = q.value(2).toString();
                                    }
                                    db.close();
                                }
                            }
                            QSqlDatabase::removeDatabase(s_syncDbConn);

                            if (!title.isEmpty()) {
                                m_calendarService.createCalendar(title, pos, color, [item, dbPath, now, pushRecursive, dIdx](bool ok, const QString &newCalId) {
                                    if (ok && !newCalId.isEmpty()) {
                                        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                        db.setDatabaseName(dbPath);
                                        if (db.open()) {
                                            QSqlQuery u(db);
                                            u.prepare(QStringLiteral("UPDATE sync_links SET remote_id = :rId, dirty = 0, synced_at = :now WHERE entity_type = 'list' AND entity_id = :id"));
                                            u.bindValue(QStringLiteral(":rId"), newCalId);
                                            u.bindValue(QStringLiteral(":now"), now);
                                            u.bindValue(QStringLiteral(":id"), item.entityId);
                                            u.exec();
                                            db.close();
                                        }
                                        QSqlDatabase::removeDatabase(s_syncDbConn);
                                    }
                                    (*pushRecursive)(dIdx + 1);
                                });
                            } else {
                                (*pushRecursive)(dIdx + 1);
                            }
                        } else if (item.dirty == 1) {
                            // Update remote calendar
                            QString title;
                            int pos = 0;
                            QString color;
                            {
                                QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                db.setDatabaseName(dbPath);
                                if (db.open()) {
                                    QSqlQuery q(db);
                                    q.prepare(QStringLiteral("SELECT name, position, color FROM lists WHERE id = :id"));
                                    q.bindValue(QStringLiteral(":id"), item.entityId);
                                    if (q.exec() && q.next()) {
                                        title = q.value(0).toString();
                                        pos = q.value(1).toInt();
                                        color = q.value(2).toString();
                                    }
                                    db.close();
                                }
                            }
                            QSqlDatabase::removeDatabase(s_syncDbConn);

                            m_calendarService.updateCalendar(item.remoteId, title, pos, color, [item, dbPath, now, pushRecursive, dIdx](bool ok) {
                                if (ok) {
                                    QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                    db.setDatabaseName(dbPath);
                                    if (db.open()) {
                                        QSqlQuery u(db);
                                        u.prepare(QStringLiteral("UPDATE sync_links SET dirty = 0, synced_at = :now WHERE entity_type = 'list' AND entity_id = :id"));
                                        u.bindValue(QStringLiteral(":now"), now);
                                        u.bindValue(QStringLiteral(":id"), item.entityId);
                                        u.exec();
                                        db.close();
                                    }
                                    QSqlDatabase::removeDatabase(s_syncDbConn);
                                }
                                (*pushRecursive)(dIdx + 1);
                            });
                        } else {
                            (*pushRecursive)(dIdx + 1);
                        }
                    } else if (item.entityType == QLatin1String("task")) {
                        // Find parent calendar remote id
                        QString calRemoteId;
                        {
                            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                            db.setDatabaseName(dbPath);
                            if (db.open()) {
                                QSqlQuery q(db);
                                q.prepare(QStringLiteral("SELECT remote_id FROM sync_links WHERE entity_type = 'list' AND entity_id = :pId"));
                                q.bindValue(QStringLiteral(":pId"), item.parentId);
                                if (q.exec() && q.next()) {
                                    calRemoteId = q.value(0).toString();
                                }
                                db.close();
                            }
                        }
                        QSqlDatabase::removeDatabase(s_syncDbConn);

                        if (calRemoteId.isEmpty()) {
                            (*pushRecursive)(dIdx + 1);
                            return;
                        }

                        if (item.deleted == 1) {
                            if (!item.remoteId.isEmpty()) {
                                m_calendarService.deleteEvent(calRemoteId, item.remoteId, [item, dbPath, pushRecursive, dIdx](bool) {
                                    QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                    db.setDatabaseName(dbPath);
                                    if (db.open()) {
                                        QSqlQuery d(db);
                                        d.prepare(QStringLiteral("DELETE FROM sync_links WHERE entity_type = 'task' AND entity_id = :id"));
                                        d.bindValue(QStringLiteral(":id"), item.entityId);
                                        d.exec();
                                        d.prepare(QStringLiteral("DELETE FROM tasks WHERE id = :id"));
                                        d.bindValue(QStringLiteral(":id"), item.entityId);
                                        d.exec();
                                        db.close();
                                    }
                                    QSqlDatabase::removeDatabase(s_syncDbConn);
                                    (*pushRecursive)(dIdx + 1);
                                });
                            } else {
                                (*pushRecursive)(dIdx + 1);
                            }
                        } else {
                            // Read task data
                            QString title;
                            QString desc;
                            QString dueDate;
                            QString dueTime;
                            QString repeatJson;
                            int completed = 0;
                            int pos = 0;

                            {
                                QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                db.setDatabaseName(dbPath);
                                if (db.open()) {
                                    QSqlQuery q(db);
                                    q.prepare(QStringLiteral("SELECT title, description, due_date, due_time, repeat_json, completed, position FROM tasks WHERE id = :id"));
                                    q.bindValue(QStringLiteral(":id"), item.entityId);
                                    if (q.exec() && q.next()) {
                                        title = q.value(0).toString();
                                        desc = q.value(1).toString();
                                        dueDate = q.value(2).toString();
                                        dueTime = q.value(3).toString();
                                        repeatJson = q.value(4).toString();
                                        completed = q.value(5).toInt();
                                        pos = q.value(6).toInt();
                                    }
                                    db.close();
                                }
                            }
                            QSqlDatabase::removeDatabase(s_syncDbConn);

                            QJsonObject taskJson;
                            taskJson.insert(QStringLiteral("id"), item.entityId);
                            taskJson.insert(QStringLiteral("title"), title);
                            taskJson.insert(QStringLiteral("description"), desc);
                            taskJson.insert(QStringLiteral("dueDate"), dueDate);
                            taskJson.insert(QStringLiteral("dueTime"), dueTime);
                            taskJson.insert(QStringLiteral("repeat_json"), repeatJson);
                            taskJson.insert(QStringLiteral("completed"), completed != 0);
                            taskJson.insert(QStringLiteral("index"), pos);
                            const QString payload = QString::fromUtf8(QJsonDocument(taskJson).toJson(QJsonDocument::Compact));

                            if (item.remoteId.isEmpty()) {
                                // Create remote event
                                m_calendarService.createEvent(calRemoteId, item.entityId, title, payload, [item, dbPath, now, pushRecursive, dIdx](bool ok, const QString &newEventId) {
                                    if (ok && !newEventId.isEmpty()) {
                                        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                        db.setDatabaseName(dbPath);
                                        if (db.open()) {
                                            QSqlQuery u(db);
                                            u.prepare(QStringLiteral("UPDATE sync_links SET remote_id = :rId, dirty = 0, synced_at = :now WHERE entity_type = 'task' AND entity_id = :id"));
                                            u.bindValue(QStringLiteral(":rId"), newEventId);
                                            u.bindValue(QStringLiteral(":now"), now);
                                            u.bindValue(QStringLiteral(":id"), item.entityId);
                                            u.exec();
                                            db.close();
                                        }
                                        QSqlDatabase::removeDatabase(s_syncDbConn);
                                    }
                                    (*pushRecursive)(dIdx + 1);
                                });
                            } else if (item.dirty == 1) {
                                // Update remote event
                                m_calendarService.updateEvent(calRemoteId, item.remoteId, item.entityId, title, payload, [item, dbPath, now, pushRecursive, dIdx](bool ok) {
                                    if (ok) {
                                        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                                        db.setDatabaseName(dbPath);
                                        if (db.open()) {
                                            QSqlQuery u(db);
                                            u.prepare(QStringLiteral("UPDATE sync_links SET dirty = 0, synced_at = :now WHERE entity_type = 'task' AND entity_id = :id"));
                                            u.bindValue(QStringLiteral(":now"), now);
                                            u.bindValue(QStringLiteral(":id"), item.entityId);
                                            u.exec();
                                            db.close();
                                        }
                                        QSqlDatabase::removeDatabase(s_syncDbConn);
                                    }
                                    (*pushRecursive)(dIdx + 1);
                                });
                            } else {
                                (*pushRecursive)(dIdx + 1);
                            }
                        }
                    } else {
                        (*pushRecursive)(dIdx + 1);
                    }
                };

                (*pushRecursive)(0);
                return;
            }

            const RemoteCalendar cal = remoteCals.at(index);
            const QString localListId = remoteCalToLocalListId.value(cal.id);

            m_calendarService.listEvents(cal.id, [this, cal, localListId, dbPath, now, fetchEventsRecursive, index](bool ok, const QList<RemoteEvent> &events) {
                if (ok && !localListId.isEmpty()) {
                    QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_syncDbConn);
                    db.setDatabaseName(dbPath);
                    if (db.open()) {
                        for (const RemoteEvent &ev : events) {
                            QString taskId = ev.iCalUID.isEmpty() ? ev.id : ev.iCalUID;
                            QString title = ev.summary;
                            QString desc;
                            QString dueDate;
                            QString dueTime;
                            QString repeatJson;
                            bool completed = false;
                            int pos = 0;

                            if (!ev.jsonPayload.isEmpty()) {
                                QJsonDocument doc = QJsonDocument::fromJson(ev.jsonPayload.toUtf8());
                                if (doc.isObject()) {
                                    QJsonObject o = doc.object();
                                    if (o.contains(QStringLiteral("id"))) {
                                        taskId = o.value(QStringLiteral("id")).toString();
                                    }
                                    if (o.contains(QStringLiteral("title"))) {
                                        title = o.value(QStringLiteral("title")).toString();
                                    }
                                    desc = o.value(QStringLiteral("description")).toString();
                                    dueDate = o.value(QStringLiteral("dueDate")).toString();
                                    dueTime = o.value(QStringLiteral("dueTime")).toString();
                                    if (o.contains(QStringLiteral("repeat_json"))) {
                                        repeatJson = o.value(QStringLiteral("repeat_json")).toString();
                                    } else if (o.contains(QStringLiteral("repeatRule"))) {
                                        repeatJson = o.value(QStringLiteral("repeatRule")).toString();
                                    }
                                    completed = o.value(QStringLiteral("completed")).toBool(false);
                                    pos = o.value(QStringLiteral("index")).toInt(0);
                                }
                            }

                            // Check local task
                            QSqlQuery check(db);
                            check.prepare(QStringLiteral("SELECT t.id, COALESCE(s.dirty, 0) FROM tasks t LEFT JOIN sync_links s ON s.entity_type = 'task' AND s.entity_id = t.id WHERE t.id = :id"));
                            check.bindValue(QStringLiteral(":id"), taskId);
                            if (check.exec() && check.next()) {
                                const int isDirty = check.value(1).toInt();
                                if (isDirty == 0) {
                                    // Update local task
                                    QSqlQuery u(db);
                                    u.prepare(QStringLiteral("UPDATE tasks SET list_id = :lId, title = :title, description = :desc, due_date = :dDate, due_time = :dTime, "
                                                            "repeat_json = :rJson, completed = :comp, position = :pos, updated_at = :now WHERE id = :id"));
                                    u.bindValue(QStringLiteral(":lId"), localListId);
                                    u.bindValue(QStringLiteral(":title"), title);
                                    u.bindValue(QStringLiteral(":desc"), desc);
                                    u.bindValue(QStringLiteral(":dDate"), dueDate);
                                    u.bindValue(QStringLiteral(":dTime"), dueTime);
                                    u.bindValue(QStringLiteral(":rJson"), repeatJson);
                                    u.bindValue(QStringLiteral(":comp"), completed ? 1 : 0);
                                    u.bindValue(QStringLiteral(":pos"), pos);
                                    u.bindValue(QStringLiteral(":now"), now);
                                    u.bindValue(QStringLiteral(":id"), taskId);
                                    u.exec();

                                    QSqlQuery sl(db);
                                    sl.prepare(QStringLiteral("UPDATE sync_links SET parent_id = :pId, remote_id = :rId, synced_at = :now WHERE entity_type = 'task' AND entity_id = :id"));
                                    sl.bindValue(QStringLiteral(":pId"), localListId);
                                    sl.bindValue(QStringLiteral(":rId"), ev.id);
                                    sl.bindValue(QStringLiteral(":now"), now);
                                    sl.bindValue(QStringLiteral(":id"), taskId);
                                    sl.exec();
                                }
                            } else {
                                // Insert new local task
                                QSqlQuery ins(db);
                                ins.prepare(QStringLiteral("INSERT INTO tasks (id, list_id, title, description, due_date, due_time, repeat_json, completed, position, updated_at) "
                                                           "VALUES (:id, :lId, :title, :desc, :dDate, :dTime, :rJson, :comp, :pos, :now)"));
                                ins.bindValue(QStringLiteral(":id"), taskId);
                                ins.bindValue(QStringLiteral(":lId"), localListId);
                                ins.bindValue(QStringLiteral(":title"), title);
                                ins.bindValue(QStringLiteral(":desc"), desc);
                                ins.bindValue(QStringLiteral(":dDate"), dueDate);
                                ins.bindValue(QStringLiteral(":dTime"), dueTime);
                                ins.bindValue(QStringLiteral(":rJson"), repeatJson);
                                ins.bindValue(QStringLiteral(":comp"), completed ? 1 : 0);
                                ins.bindValue(QStringLiteral(":pos"), pos);
                                ins.bindValue(QStringLiteral(":now"), now);
                                ins.exec();

                                QSqlQuery sl(db);
                                sl.prepare(QStringLiteral("INSERT OR REPLACE INTO sync_links (entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at) "
                                                          "VALUES ('task', :id, :pId, :rId, 0, 0, :now, :now)"));
                                sl.bindValue(QStringLiteral(":id"), taskId);
                                sl.bindValue(QStringLiteral(":pId"), localListId);
                                sl.bindValue(QStringLiteral(":rId"), ev.id);
                                sl.bindValue(QStringLiteral(":now"), now);
                                sl.exec();
                            }
                        }
                        db.close();
                    }
                    QSqlDatabase::removeDatabase(s_syncDbConn);
                }

                (*fetchEventsRecursive)(index + 1);
            });
        };

        (*fetchEventsRecursive)(0);
    });
}
