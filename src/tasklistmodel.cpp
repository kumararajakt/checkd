#include "tasklistmodel.h"

#include <QDateTime>
#include <QDebug>
#include <QSqlError>
#include <QSqlField>
#include <QSqlQuery>
#include <QUuid>

#include "asyncsql/databaseexception.h"
#include "notificationmanager.h"
#include "syncmanager.h"

using namespace AsyncSql;

TaskListModel::TaskListModel(QObject *parent)
    : AsyncSqlListModel(parent)
{
    setTable(QStringLiteral("tasks"));

    connect(this, &AsyncSqlBaseModel::selected, this, &TaskListModel::countChanged);
    connect(this, &QAbstractItemModel::modelReset, this, &TaskListModel::countChanged);
    connect(this, &QAbstractItemModel::rowsInserted, this, &TaskListModel::countChanged);
    connect(this, &QAbstractItemModel::rowsRemoved, this, &TaskListModel::countChanged);
    connect(&NotificationManager::instance(), &NotificationManager::taskStateChanged, this, &TaskListModel::reload);
    connect(&SyncManager::instance(), &SyncManager::dataReloadRequested, this, &TaskListModel::reload);
}

void TaskListModel::onInit(QSqlDatabase &db)
{
    QSqlQuery q(db);
    q.exec(QStringLiteral("PRAGMA foreign_keys = ON;"));
}

QString TaskListModel::listId() const
{
    return m_listId;
}

void TaskListModel::setListId(const QString &id)
{
    if (m_listId == id) {
        return;
    }

    m_listId = id;
    Q_EMIT listIdChanged();

    if (m_listId.isEmpty()) {
        beginResetModel();
        records_.clear();
        originalRecords_.clear();
        m_repeatRuleCache.clear();
        endResetModel();
        return;
    }

    setTable(QStringLiteral("tasks"));
    const QString escapedListId = QString(m_listId).replace(QLatin1Char('\''), QStringLiteral("''"));
    setSelectQuery(QStringLiteral("SELECT * FROM tasks WHERE list_id = '%1' ORDER BY position ASC").arg(escapedListId));
    select();
}

QHash<int, QByteArray> TaskListModel::roleNames() const
{
    static const QHash<int, QByteArray> roles = {
        { IdRole, "id" },
        { ListIdRole, "listId" },
        { TitleRole, "title" },
        { DescriptionRole, "description" },
        { DueDateRole, "dueDate" },
        { DueTimeRole, "dueTime" },
        { RepeatJsonRole, "repeat_json" },
        { RepeatConfigRole, "repeatConfig" },
        { RepeatRuleRole, "repeatRule" },
        { PositionRole, "position" },
        { UpdatedAtRole, "updated_at" },
        { CompletedRole, "completed" },
        { IsCompletedRole, "isCompleted" },
        { NextOccurrenceRole, "next_occurrence" },
        { OccurrenceIndexRole, "occurrence_index" },
        { NotifiedKeyRole, "notified_key" }
    };
    return roles;
}

int TaskListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid()) {
        return 0;
    }
    return records_.count();
}

QString TaskListModel::formatRepeatSummary(const QString &repeatJson)
{
    if (repeatJson.trimmed().isEmpty()) {
        return QString();
    }

    QJsonDocument doc = QJsonDocument::fromJson(repeatJson.toUtf8());
    if (!doc.isObject()) {
        return QString();
    }

    QJsonObject obj = doc.object();
    QString type = obj.value(QStringLiteral("type")).toString();
    if (type.isEmpty()) {
        type = obj.value(QStringLiteral("unit")).toString();
    }

    int interval = obj.value(QStringLiteral("interval")).toInt(1);
    if (interval < 1) {
        interval = 1;
    }

    QString summary;
    if (type == QStringLiteral("day")) {
        summary = (interval == 1) ? QStringLiteral("Daily") : QStringLiteral("Every %1 days").arg(interval);
    } else if (type == QStringLiteral("week")) {
        summary = (interval == 1) ? QStringLiteral("Weekly") : QStringLiteral("Every %1 weeks").arg(interval);
    } else if (type == QStringLiteral("month")) {
        summary = (interval == 1) ? QStringLiteral("Monthly") : QStringLiteral("Every %1 months").arg(interval);
    } else if (type == QStringLiteral("year")) {
        summary = (interval == 1) ? QStringLiteral("Yearly") : QStringLiteral("Every %1 years").arg(interval);
    }

    QString time = obj.value(QStringLiteral("time")).toString();
    if (!time.isEmpty() && !summary.isEmpty()) {
        summary += QStringLiteral(" at %1").arg(time);
    }

    return summary;
}

QVariant TaskListModel::data(const QModelIndex &index, int role) const
{
    if (records_.isEmpty() || !index.isValid() || index.row() < 0 || index.row() >= records_.count()) {
        return QVariant();
    }

    const QSqlRecord &rec = records_.at(index.row());
    const QString id = rec.value(QStringLiteral("id")).toString();

    switch (role) {
    case IdRole:
        return id;
    case ListIdRole:
        return rec.value(QStringLiteral("list_id"));
    case TitleRole:
    case Qt::DisplayRole:
    case Qt::EditRole:
        return rec.value(QStringLiteral("title"));
    case DescriptionRole:
        return rec.value(QStringLiteral("description"));
    case DueDateRole:
        return rec.value(QStringLiteral("due_date"));
    case DueTimeRole:
        return rec.value(QStringLiteral("due_time"));
    case RepeatJsonRole:
    case RepeatConfigRole:
        return rec.value(QStringLiteral("repeat_json"));
    case RepeatRuleRole: {
        auto it = m_repeatRuleCache.find(id);
        if (it != m_repeatRuleCache.end()) {
            return it.value();
        }
        return formatRepeatSummary(rec.value(QStringLiteral("repeat_json")).toString());
    }
    case PositionRole:
        return rec.value(QStringLiteral("position"));
    case UpdatedAtRole:
        return rec.value(QStringLiteral("updated_at"));
    case CompletedRole:
    case IsCompletedRole:
        return rec.value(QStringLiteral("completed")).toInt() != 0;
    case NextOccurrenceRole:
        return rec.value(QStringLiteral("next_occurrence"));
    case OccurrenceIndexRole:
        return rec.value(QStringLiteral("occurrence_index"));
    case NotifiedKeyRole:
        return rec.value(QStringLiteral("notified_key"));
    default:
        return AsyncSqlListModel::data(index, role);
    }
}

bool TaskListModel::setData(const QModelIndex &index, const QVariant &value, int role)
{
    if (!index.isValid() || index.row() < 0 || index.row() >= records_.count()) {
        return false;
    }

    switch (role) {
    case TitleRole:
    case Qt::EditRole:
        setTitle(index.row(), value.toString());
        return true;
    case DescriptionRole:
        setDescription(index.row(), value.toString());
        return true;
    case DueDateRole:
        setDueDate(index.row(), value.toString());
        return true;
    case DueTimeRole:
        setDueTime(index.row(), value.toString());
        return true;
    case CompletedRole:
    case IsCompletedRole:
        setCompleted(index.row(), value.toBool());
        return true;
    case RepeatConfigRole:
    case RepeatJsonRole:
        setRepeat(index.row(), QString(), value.toString());
        return true;
    case RepeatRuleRole: {
        const QString id = records_.at(index.row()).value(QStringLiteral("id")).toString();
        m_repeatRuleCache[id] = value.toString();
        Q_EMIT dataChanged(index, index, { RepeatRuleRole });
        return true;
    }
    default:
        return AsyncSqlListModel::setData(index, value, role);
    }
}

QVariantMap TaskListModel::get(int row) const
{
    if (row < 0 || row >= records_.count()) {
        return {};
    }

    const QSqlRecord &rec = records_.at(row);
    const QString id = rec.value(QStringLiteral("id")).toString();
    QVariantMap map;
    map.insert(QStringLiteral("id"), id);
    map.insert(QStringLiteral("list_id"), rec.value(QStringLiteral("list_id")));
    map.insert(QStringLiteral("listId"), rec.value(QStringLiteral("list_id")));
    map.insert(QStringLiteral("title"), rec.value(QStringLiteral("title")));
    map.insert(QStringLiteral("description"), rec.value(QStringLiteral("description")));
    map.insert(QStringLiteral("dueDate"), rec.value(QStringLiteral("due_date")));
    map.insert(QStringLiteral("due_date"), rec.value(QStringLiteral("due_date")));
    map.insert(QStringLiteral("dueTime"), rec.value(QStringLiteral("due_time")));
    map.insert(QStringLiteral("due_time"), rec.value(QStringLiteral("due_time")));
    map.insert(QStringLiteral("repeatConfig"), rec.value(QStringLiteral("repeat_json")));
    map.insert(QStringLiteral("repeat_json"), rec.value(QStringLiteral("repeat_json")));

    QString repeatRule = m_repeatRuleCache.value(id);
    if (repeatRule.isEmpty()) {
        repeatRule = formatRepeatSummary(rec.value(QStringLiteral("repeat_json")).toString());
    }
    map.insert(QStringLiteral("repeatRule"), repeatRule);
    map.insert(QStringLiteral("isCompleted"), rec.value(QStringLiteral("completed")).toInt() != 0);
    map.insert(QStringLiteral("completed"), rec.value(QStringLiteral("completed")).toInt() != 0);
    map.insert(QStringLiteral("position"), rec.value(QStringLiteral("position")));
    map.insert(QStringLiteral("updated_at"), rec.value(QStringLiteral("updated_at")));
    map.insert(QStringLiteral("updatedAt"), rec.value(QStringLiteral("updated_at")));
    return map;
}

bool TaskListModel::setProperty(int row, const QString &property, const QVariant &value)
{
    if (row < 0 || row >= records_.count()) {
        return false;
    }

    if (property == QStringLiteral("title")) {
        setTitle(row, value.toString());
        return true;
    } else if (property == QStringLiteral("description")) {
        setDescription(row, value.toString());
        return true;
    } else if (property == QStringLiteral("dueDate") || property == QStringLiteral("due_date")) {
        setDueDate(row, value.toString());
        return true;
    } else if (property == QStringLiteral("dueTime") || property == QStringLiteral("due_time")) {
        setDueTime(row, value.toString());
        return true;
    } else if (property == QStringLiteral("isCompleted") || property == QStringLiteral("completed")) {
        setCompleted(row, value.toBool());
        return true;
    } else if (property == QStringLiteral("repeatConfig") || property == QStringLiteral("repeat_json")) {
        setRepeat(row, QString(), value.toString());
        return true;
    } else if (property == QStringLiteral("repeatRule") || property == QStringLiteral("repeat_rule")) {
        const QString id = records_.at(row).value(QStringLiteral("id")).toString();
        m_repeatRuleCache[id] = value.toString();
        const QModelIndex idx = index(row, 0);
        Q_EMIT dataChanged(idx, idx, { RepeatRuleRole });
        return true;
    }

    return false;
}

void TaskListModel::addTask(const QString &title,
                           const QString &description,
                           const QString &dueDate,
                           const QString &dueTime,
                           const QString &repeatJson)
{
    const QString trimmedTitle = title.trimmed();
    if (trimmedTitle.isEmpty() || m_listId.isEmpty()) {
        return;
    }

    const QString id = QUuid::createUuid().toString(QUuid::WithoutBraces);
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    int maxPos = 0;
    for (const QSqlRecord &rec : std::as_const(records_)) {
        int pos = rec.value(QStringLiteral("position")).toInt();
        if (pos > maxPos) {
            maxPos = pos;
        }
    }
    const int newPos = maxPos + 1;

    const QString desc = description.isNull() ? QStringLiteral("") : description;

    QSqlRecord newRecord = emptyRecord_;
    if (newRecord.isEmpty()) {
        newRecord.append(QSqlField(QStringLiteral("id"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("list_id"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("title"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("description"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("due_date"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("due_time"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("repeat_json"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("position"), QMetaType(QMetaType::Int)));
        newRecord.append(QSqlField(QStringLiteral("updated_at"), QMetaType(QMetaType::LongLong)));
        newRecord.append(QSqlField(QStringLiteral("completed"), QMetaType(QMetaType::Int)));
        newRecord.append(QSqlField(QStringLiteral("next_occurrence"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("occurrence_index"), QMetaType(QMetaType::LongLong)));
        newRecord.append(QSqlField(QStringLiteral("notified_key"), QMetaType(QMetaType::QString)));
    }
    newRecord.setValue(QStringLiteral("id"), id);
    newRecord.setValue(QStringLiteral("list_id"), m_listId);
    newRecord.setValue(QStringLiteral("title"), trimmedTitle);
    newRecord.setValue(QStringLiteral("description"), desc);
    newRecord.setValue(QStringLiteral("due_date"), dueDate);
    newRecord.setValue(QStringLiteral("due_time"), dueTime);
    newRecord.setValue(QStringLiteral("repeat_json"), repeatJson);
    newRecord.setValue(QStringLiteral("position"), newPos);
    newRecord.setValue(QStringLiteral("updated_at"), now);
    newRecord.setValue(QStringLiteral("completed"), 0);

    const int newRow = records_.count();
    beginInsertRows(QModelIndex(), newRow, newRow);
    records_.append(newRecord);
    originalRecords_.append(newRecord);
    endInsertRows();

    const QString listId = m_listId;
    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, trimmedTitle, desc, dueDate, dueTime, repeatJson, newPos, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral(
            "INSERT INTO tasks (id, list_id, title, description, due_date, due_time, repeat_json, position, updated_at, completed) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)"
        ));
        q.addBindValue(id);
        q.addBindValue(listId);
        q.addBindValue(trimmedTitle);
        q.addBindValue(desc);
        q.addBindValue(dueDate.isEmpty() ? QVariant(QMetaType(QMetaType::QString)) : dueDate);
        q.addBindValue(dueTime.isEmpty() ? QVariant(QMetaType(QMetaType::QString)) : dueTime);
        q.addBindValue(repeatJson.isEmpty() ? QVariant(QMetaType(QMetaType::QString)) : repeatJson);
        q.addBindValue(newPos);
        q.addBindValue(now);
        if (!q.exec()) {
            qWarning() << "Failed to insert task:" << q.lastError().text();
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "parent_id = excluded.parent_id, dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            qWarning() << "Failed to insert sync_link for task:" << sq.lastError().text();
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::setTitle(int row, const QString &title)
{
    const QString trimmed = title.trimmed();
    if (trimmed.isEmpty() || row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("title"), trimmed);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { TitleRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, trimmed, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE tasks SET title = ?, updated_at = ? WHERE id = ?"));
        q.addBindValue(trimmed);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::setDescription(int row, const QString &description)
{
    if (row < 0 || row >= records_.count()) {
        return;
    }

    const QString desc = description.isNull() ? QStringLiteral("") : description;
    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("description"), desc);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { DescriptionRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, desc, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE tasks SET description = ?, updated_at = ? WHERE id = ?"));
        q.addBindValue(desc);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::setDueDate(int row, const QString &dueDate)
{
    if (row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("due_date"), dueDate);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { DueDateRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, dueDate, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE tasks SET due_date = ?, notified_key = NULL, updated_at = ? WHERE id = ?"));
        q.addBindValue(dueDate.isEmpty() ? QVariant(QMetaType(QMetaType::QString)) : dueDate);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::setDueTime(int row, const QString &dueTime)
{
    if (row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("due_time"), dueTime);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { DueTimeRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, dueTime, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE tasks SET due_time = ?, notified_key = NULL, updated_at = ? WHERE id = ?"));
        q.addBindValue(dueTime.isEmpty() ? QVariant(QMetaType(QMetaType::QString)) : dueTime);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::setCompleted(int row, bool completed)
{
    if (row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    const int completedInt = completed ? 1 : 0;

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("completed"), completedInt);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { CompletedRole, IsCompletedRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, completedInt, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE tasks SET completed = ?, updated_at = ? WHERE id = ?"));
        q.addBindValue(completedInt);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::setRepeat(int row, const QString &repeatRule, const QString &repeatJson)
{
    if (row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("repeat_json"), repeatJson);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    if (!repeatRule.isEmpty()) {
        m_repeatRuleCache[id] = repeatRule;
    } else {
        m_repeatRuleCache[id] = formatRepeatSummary(repeatJson);
    }

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { RepeatJsonRole, RepeatConfigRole, RepeatRuleRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, repeatJson, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE tasks SET repeat_json = ?, notified_key = NULL, next_occurrence = NULL, updated_at = ? WHERE id = ?"));
        q.addBindValue(repeatJson.isEmpty() ? QVariant(QMetaType(QMetaType::QString)) : repeatJson);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

bool TaskListModel::remove(int row)
{
    if (row < 0 || row >= records_.count()) {
        return false;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const QString listId = records_.at(row).value(QStringLiteral("list_id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    beginRemoveRows(QModelIndex(), row, row);
    records_.removeAt(row);
    originalRecords_ = records_;
    m_repeatRuleCache.remove(id);
    endRemoveRows();

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, listId, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("DELETE FROM tasks WHERE id = ?"));
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('task', ?, ?, 1, 1, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "parent_id = COALESCE(excluded.parent_id, sync_links.parent_id), "
            "dirty = 1, deleted = 1, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(listId);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
    return true;
}

void TaskListModel::deleteTask(int row)
{
    remove(row);
}

void TaskListModel::move(int from, int to)
{
    if (from < 0 || from >= records_.count() || to < 0 || to >= records_.count() || from == to) {
        return;
    }

    const int destIndex = (to > from) ? to + 1 : to;
    if (!beginMoveRows(QModelIndex(), from, from, QModelIndex(), destIndex)) {
        return;
    }

    records_.move(from, to);
    endMoveRows();

    QList<QPair<QString, int>> updates;
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    for (int i = 0; i < records_.count(); ++i) {
        QSqlRecord rec = records_.at(i);
        const int pos = i + 1;
        rec.setValue(QStringLiteral("position"), pos);
        rec.setValue(QStringLiteral("updated_at"), now);
        records_[i] = rec;
        updates.append({ rec.value(QStringLiteral("id")).toString(), pos });
    }
    originalRecords_ = records_;

    const QString listId = m_listId;
    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([updates, listId, now](QSqlDatabase db) {
        for (const auto &item : updates) {
            QSqlQuery q(db);
            q.prepare(QStringLiteral("UPDATE tasks SET position = ?, updated_at = ? WHERE id = ?"));
            q.addBindValue(item.second);
            q.addBindValue(now);
            q.addBindValue(item.first);
            q.exec();

            QSqlQuery sq(db);
            sq.prepare(QStringLiteral(
                "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
                "VALUES ('task', ?, ?, 1, 0, ?) "
                "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
                "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
            ));
            sq.addBindValue(item.first);
            sq.addBindValue(listId);
            sq.addBindValue(now);
            sq.exec();
        }
    });

    Q_EMIT execute(request);
}

void TaskListModel::reload()
{
    if (!m_listId.isEmpty()) {
        select();
    }
}

bool TaskListModel::getResults(const AsyncSql::QueryResult &result)
{
    const bool ok = AsyncSqlListModel::getResults(result);
    if (ok) {
        Q_EMIT countChanged();
    }
    return ok;
}
