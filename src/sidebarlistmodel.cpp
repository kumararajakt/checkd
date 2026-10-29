#include "sidebarlistmodel.h"

#include <QDateTime>
#include <QDebug>
#include <QSqlError>
#include <QSqlField>
#include <QSqlQuery>
#include <QUuid>

#include "asyncsql/databaseexception.h"

using namespace AsyncSql;

SidebarListModel::SidebarListModel(QObject *parent)
    : AsyncSqlListModel(parent)
{
    setTable(QStringLiteral("lists"));
    setSelectQuery(QStringLiteral("SELECT * FROM lists ORDER BY position ASC"));

    connect(this, &AsyncSqlBaseModel::selected, this, &SidebarListModel::countChanged);
    connect(this, &QAbstractItemModel::modelReset, this, &SidebarListModel::countChanged);
    connect(this, &QAbstractItemModel::rowsInserted, this, &SidebarListModel::countChanged);
    connect(this, &QAbstractItemModel::rowsRemoved, this, &SidebarListModel::countChanged);

    select();
}

void SidebarListModel::onInit(QSqlDatabase &db)
{
    QSqlQuery q(db);
    q.exec(QStringLiteral("PRAGMA foreign_keys = ON;"));
}

QHash<int, QByteArray> SidebarListModel::roleNames() const
{
    static const QHash<int, QByteArray> roles = {
        { IdRole, "id" },
        { NameRole, "name" },
        { LabelRole, "label" },
        { ColorRole, "color" },
        { ModelColorRole, "modelColor" },
        { PositionRole, "position" },
        { UpdatedAtRole, "updated_at" }
    };
    return roles;
}

int SidebarListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid()) {
        return 0;
    }
    return records_.count();
}

QVariant SidebarListModel::data(const QModelIndex &index, int role) const
{
    if (records_.isEmpty() || !index.isValid() || index.row() < 0 || index.row() >= records_.count()) {
        return QVariant();
    }

    const QSqlRecord &rec = records_.at(index.row());

    switch (role) {
    case IdRole:
        return rec.value(QStringLiteral("id"));
    case NameRole:
    case LabelRole:
    case Qt::DisplayRole:
    case Qt::EditRole:
        return rec.value(QStringLiteral("name"));
    case ColorRole:
    case ModelColorRole:
        return rec.value(QStringLiteral("color"));
    case PositionRole:
        return rec.value(QStringLiteral("position"));
    case UpdatedAtRole:
        return rec.value(QStringLiteral("updated_at"));
    default:
        return AsyncSqlListModel::data(index, role);
    }
}

bool SidebarListModel::setData(const QModelIndex &index, const QVariant &value, int role)
{
    if (!index.isValid() || index.row() < 0 || index.row() >= records_.count()) {
        return false;
    }

    if (role == NameRole || role == LabelRole || role == Qt::EditRole) {
        renameList(index.row(), value.toString());
        return true;
    } else if (role == ColorRole || role == ModelColorRole) {
        setListColor(index.row(), value.toString());
        return true;
    }

    return AsyncSqlListModel::setData(index, value, role);
}

QVariantMap SidebarListModel::get(int row) const
{
    if (row < 0 || row >= records_.count()) {
        return {};
    }

    const QSqlRecord &rec = records_.at(row);
    QVariantMap map;
    map.insert(QStringLiteral("id"), rec.value(QStringLiteral("id")));
    map.insert(QStringLiteral("name"), rec.value(QStringLiteral("name")));
    map.insert(QStringLiteral("label"), rec.value(QStringLiteral("name")));
    map.insert(QStringLiteral("color"), rec.value(QStringLiteral("color")));
    map.insert(QStringLiteral("modelColor"), rec.value(QStringLiteral("color")));
    map.insert(QStringLiteral("position"), rec.value(QStringLiteral("position")));
    map.insert(QStringLiteral("updated_at"), rec.value(QStringLiteral("updated_at")));
    map.insert(QStringLiteral("updatedAt"), rec.value(QStringLiteral("updated_at")));
    return map;
}

int SidebarListModel::indexOfId(const QString &id) const
{
    if (id.isEmpty()) {
        return -1;
    }

    for (int i = 0; i < records_.count(); ++i) {
        if (records_.at(i).value(QStringLiteral("id")).toString() == id) {
            return i;
        }
    }
    return -1;
}

bool SidebarListModel::setProperty(int row, const QString &property, const QVariant &value)
{
    if (row < 0 || row >= records_.count()) {
        return false;
    }

    if (property == QStringLiteral("name") || property == QStringLiteral("label")) {
        renameList(row, value.toString());
        return true;
    } else if (property == QStringLiteral("color") || property == QStringLiteral("modelColor")) {
        setListColor(row, value.toString());
        return true;
    }
    return false;
}

void SidebarListModel::addList(const QString &name, const QString &color)
{
    const QString trimmedName = name.trimmed();
    if (trimmedName.isEmpty()) {
        return;
    }

    const QString id = QUuid::createUuid().toString(QUuid::WithoutBraces);
    const QString colorVal = color.trimmed().isEmpty() ? QStringLiteral("#3b82f6") : color.trimmed();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    int maxPos = 0;
    for (const QSqlRecord &rec : std::as_const(records_)) {
        int pos = rec.value(QStringLiteral("position")).toInt();
        if (pos > maxPos) {
            maxPos = pos;
        }
    }
    const int newPos = maxPos + 1;

    QSqlRecord newRecord = emptyRecord_;
    if (newRecord.isEmpty()) {
        newRecord.append(QSqlField(QStringLiteral("id"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("name"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("color"), QMetaType(QMetaType::QString)));
        newRecord.append(QSqlField(QStringLiteral("position"), QMetaType(QMetaType::Int)));
        newRecord.append(QSqlField(QStringLiteral("updated_at"), QMetaType(QMetaType::LongLong)));
    }
    newRecord.setValue(QStringLiteral("id"), id);
    newRecord.setValue(QStringLiteral("name"), trimmedName);
    newRecord.setValue(QStringLiteral("color"), colorVal);
    newRecord.setValue(QStringLiteral("position"), newPos);
    newRecord.setValue(QStringLiteral("updated_at"), now);

    const int newRow = records_.count();
    beginInsertRows(QModelIndex(), newRow, newRow);
    records_.append(newRecord);
    originalRecords_.append(newRecord);
    endInsertRows();

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, trimmedName, colorVal, newPos, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral(
            "INSERT INTO lists (id, name, color, position, updated_at) "
            "VALUES (?, ?, ?, ?, ?)"
        ));
        q.addBindValue(id);
        q.addBindValue(trimmedName);
        q.addBindValue(colorVal);
        q.addBindValue(newPos);
        q.addBindValue(now);
        if (!q.exec()) {
            qWarning() << "Failed to insert list:" << q.lastError().text();
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('list', ?, NULL, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(now);
        if (!sq.exec()) {
            qWarning() << "Failed to insert sync_link for list:" << sq.lastError().text();
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void SidebarListModel::renameList(int row, const QString &name)
{
    const QString trimmedName = name.trimmed();
    if (trimmedName.isEmpty() || row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("name"), trimmedName);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { NameRole, LabelRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, trimmedName, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE lists SET name = ?, updated_at = ? WHERE id = ?"));
        q.addBindValue(trimmedName);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('list', ?, NULL, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

void SidebarListModel::setListColor(int row, const QString &color)
{
    const QString trimmedColor = color.trimmed();
    if (trimmedColor.isEmpty() || row < 0 || row >= records_.count()) {
        return;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    QSqlRecord rec = records_.at(row);
    rec.setValue(QStringLiteral("color"), trimmedColor);
    rec.setValue(QStringLiteral("updated_at"), now);
    records_[row] = rec;
    originalRecords_ = records_;

    const QModelIndex idx = index(row, 0);
    Q_EMIT dataChanged(idx, idx, { ColorRole, ModelColorRole, UpdatedAtRole });

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, trimmedColor, now](QSqlDatabase db) {
        QSqlQuery q(db);
        q.prepare(QStringLiteral("UPDATE lists SET color = ?, updated_at = ? WHERE id = ?"));
        q.addBindValue(trimmedColor);
        q.addBindValue(now);
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('list', ?, NULL, 1, 0, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }
    });

    Q_EMIT execute(request);
}

bool SidebarListModel::remove(int row)
{
    if (row < 0 || row >= records_.count()) {
        return false;
    }

    const QString id = records_.at(row).value(QStringLiteral("id")).toString();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    beginRemoveRows(QModelIndex(), row, row);
    records_.removeAt(row);
    originalRecords_ = records_;
    endRemoveRows();

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([id, now](QSqlDatabase db) {
        QSqlQuery tq(db);
        tq.prepare(QStringLiteral("SELECT id FROM tasks WHERE list_id = ?"));
        tq.addBindValue(id);
        QStringList taskIds;
        if (tq.exec()) {
            while (tq.next()) {
                taskIds << tq.value(0).toString();
            }
        }

        QSqlQuery q(db);
        q.prepare(QStringLiteral("DELETE FROM lists WHERE id = ?"));
        q.addBindValue(id);
        if (!q.exec()) {
            throw DatabaseException(q.lastError());
        }

        QSqlQuery sq(db);
        sq.prepare(QStringLiteral(
            "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
            "VALUES ('list', ?, NULL, 1, 1, ?) "
            "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
            "dirty = 1, deleted = 1, updated_at = excluded.updated_at"
        ));
        sq.addBindValue(id);
        sq.addBindValue(now);
        if (!sq.exec()) {
            throw DatabaseException(sq.lastError());
        }

        for (const QString &taskId : taskIds) {
            QSqlQuery stq(db);
            stq.prepare(QStringLiteral(
                "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
                "VALUES ('task', ?, ?, 1, 1, ?) "
                "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
                "parent_id = COALESCE(excluded.parent_id, sync_links.parent_id), "
                "dirty = 1, deleted = 1, updated_at = excluded.updated_at"
            ));
            stq.addBindValue(taskId);
            stq.addBindValue(id);
            stq.addBindValue(now);
            stq.exec();
        }
    });

    Q_EMIT execute(request);
    return true;
}

void SidebarListModel::deleteList(int row)
{
    remove(row);
}

void SidebarListModel::move(int from, int to)
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

    QueryRequest request(this, QString(), tableName_, QueryRequest::CustomOperation);
    request.setCustomOperation([updates, now](QSqlDatabase db) {
        for (const auto &item : updates) {
            QSqlQuery q(db);
            q.prepare(QStringLiteral("UPDATE lists SET position = ?, updated_at = ? WHERE id = ?"));
            q.addBindValue(item.second);
            q.addBindValue(now);
            q.addBindValue(item.first);
            q.exec();

            QSqlQuery sq(db);
            sq.prepare(QStringLiteral(
                "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at) "
                "VALUES ('list', ?, NULL, 1, 0, ?) "
                "ON CONFLICT(entity_type, entity_id) DO UPDATE SET "
                "dirty = 1, deleted = 0, updated_at = excluded.updated_at"
            ));
            sq.addBindValue(item.first);
            sq.addBindValue(now);
            sq.exec();
        }
    });

    Q_EMIT execute(request);
}

void SidebarListModel::reload()
{
    select();
}

bool SidebarListModel::getResults(const AsyncSql::QueryResult &result)
{
    const bool ok = AsyncSqlListModel::getResults(result);
    if (ok) {
        Q_EMIT countChanged();
    }
    return ok;
}
