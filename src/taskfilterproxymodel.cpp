#include "taskfilterproxymodel.h"

TaskFilterProxyModel::TaskFilterProxyModel(QObject *parent)
    : QSortFilterProxyModel(parent)
{
    setDynamicSortFilter(true);
    setFilterRole(TaskListModel::CompletedRole);

    connect(this, &QAbstractItemModel::rowsInserted, this, &TaskFilterProxyModel::countChanged);
    connect(this, &QAbstractItemModel::rowsRemoved, this, &TaskFilterProxyModel::countChanged);
    connect(this, &QAbstractItemModel::modelReset, this, &TaskFilterProxyModel::countChanged);
    connect(this, &QAbstractItemModel::layoutChanged, this, &TaskFilterProxyModel::countChanged);
}

TaskListModel *TaskFilterProxyModel::sourceTaskListModel() const
{
    return m_sourceTaskListModel;
}

void TaskFilterProxyModel::setSourceTaskListModel(TaskListModel *model)
{
    if (m_sourceTaskListModel == model) {
        return;
    }

    if (m_sourceTaskListModel) {
        disconnect(m_sourceTaskListModel, &QAbstractItemModel::dataChanged,
                   this, &TaskFilterProxyModel::onSourceDataChanged);
    }

    m_sourceTaskListModel = model;
    setSourceModel(model);

    if (m_sourceTaskListModel) {
        connect(m_sourceTaskListModel, &QAbstractItemModel::dataChanged,
                this, &TaskFilterProxyModel::onSourceDataChanged);
    }

    Q_EMIT sourceTaskListModelChanged();
    Q_EMIT countChanged();
}

bool TaskFilterProxyModel::filterCompleted() const
{
    return m_filterCompleted;
}

void TaskFilterProxyModel::setFilterCompleted(bool completed)
{
    if (m_filterCompleted == completed) {
        return;
    }

    m_filterCompleted = completed;
#if QT_VERSION >= QT_VERSION_CHECK(6, 10, 0)
    beginFilterChange();
    endFilterChange(QSortFilterProxyModel::Direction::Rows);
#else
    invalidateFilter();
#endif
    Q_EMIT filterCompletedChanged();
    Q_EMIT countChanged();
}

int TaskFilterProxyModel::rowCount(const QModelIndex &parent) const
{
    return QSortFilterProxyModel::rowCount(parent);
}

bool TaskFilterProxyModel::filterAcceptsRow(int source_row, const QModelIndex &source_parent) const
{
    if (!sourceModel()) {
        return false;
    }

    const QModelIndex idx = sourceModel()->index(source_row, 0, source_parent);
    const bool isCompleted = idx.data(TaskListModel::CompletedRole).toBool();
    return m_filterCompleted ? isCompleted : !isCompleted;
}

void TaskFilterProxyModel::onSourceDataChanged(const QModelIndex &topLeft, const QModelIndex &bottomRight, const QList<int> &roles)
{
    Q_UNUSED(topLeft)
    Q_UNUSED(bottomRight)
    if (roles.isEmpty() || roles.contains(TaskListModel::CompletedRole) || roles.contains(TaskListModel::IsCompletedRole)) {
#if QT_VERSION >= QT_VERSION_CHECK(6, 10, 0)
        beginFilterChange();
        endFilterChange(QSortFilterProxyModel::Direction::Rows);
#else
        invalidateFilter();
#endif
        Q_EMIT countChanged();
    }
}

QVariantMap TaskFilterProxyModel::get(int row) const
{
    if (!m_sourceTaskListModel || row < 0 || row >= rowCount()) {
        return {};
    }
    const QModelIndex proxyIdx = index(row, 0);
    const QModelIndex srcIdx = mapToSource(proxyIdx);
    return m_sourceTaskListModel->get(srcIdx.row());
}

bool TaskFilterProxyModel::setProperty(int row, const QString &property, const QVariant &value)
{
    if (!m_sourceTaskListModel || row < 0 || row >= rowCount()) {
        return false;
    }
    const QModelIndex proxyIdx = index(row, 0);
    const QModelIndex srcIdx = mapToSource(proxyIdx);
    return m_sourceTaskListModel->setProperty(srcIdx.row(), property, value);
}

bool TaskFilterProxyModel::remove(int row)
{
    if (!m_sourceTaskListModel || row < 0 || row >= rowCount()) {
        return false;
    }
    const QModelIndex proxyIdx = index(row, 0);
    const QModelIndex srcIdx = mapToSource(proxyIdx);
    return m_sourceTaskListModel->remove(srcIdx.row());
}

void TaskFilterProxyModel::deleteTask(int row)
{
    if (!m_sourceTaskListModel || row < 0 || row >= rowCount()) {
        return;
    }
    const QModelIndex proxyIdx = index(row, 0);
    const QModelIndex srcIdx = mapToSource(proxyIdx);
    m_sourceTaskListModel->deleteTask(srcIdx.row());
}

void TaskFilterProxyModel::setCompleted(int row, bool completed)
{
    if (!m_sourceTaskListModel || row < 0 || row >= rowCount()) {
        return;
    }
    const QModelIndex proxyIdx = index(row, 0);
    const QModelIndex srcIdx = mapToSource(proxyIdx);
    m_sourceTaskListModel->setCompleted(srcIdx.row(), completed);
}

bool TaskFilterProxyModel::moveToList(int row, const QString &targetListId)
{
    if (!m_sourceTaskListModel || row < 0 || row >= rowCount()) {
        return false;
    }
    const QModelIndex proxyIdx = index(row, 0);
    const QModelIndex srcIdx = mapToSource(proxyIdx);
    return m_sourceTaskListModel->moveToList(srcIdx.row(), targetListId);
}

QVariantList TaskFilterProxyModel::otherLists() const
{
    if (m_sourceTaskListModel) {
        return m_sourceTaskListModel->otherLists();
    }
    return {};
}

void TaskFilterProxyModel::addTask(const QString &title,
                                   const QString &description,
                                   const QString &dueDate,
                                   const QString &dueTime,
                                   const QString &repeatJson)
{
    if (m_sourceTaskListModel) {
        m_sourceTaskListModel->addTask(title, description, dueDate, dueTime, repeatJson);
    }
}
