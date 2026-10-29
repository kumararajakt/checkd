#ifndef TASKFILTERPROXYMODEL_H
#define TASKFILTERPROXYMODEL_H

#include <QSortFilterProxyModel>
#include <QtQml/qqmlregistration.h>
#include "tasklistmodel.h"

class TaskFilterProxyModel : public QSortFilterProxyModel
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(TaskListModel *sourceTaskListModel READ sourceTaskListModel WRITE setSourceTaskListModel NOTIFY sourceTaskListModelChanged)
    Q_PROPERTY(bool filterCompleted READ filterCompleted WRITE setFilterCompleted NOTIFY filterCompletedChanged)
    Q_PROPERTY(int count READ rowCount NOTIFY countChanged)

public:
    explicit TaskFilterProxyModel(QObject *parent = nullptr);
    ~TaskFilterProxyModel() override = default;

    TaskListModel *sourceTaskListModel() const;
    void setSourceTaskListModel(TaskListModel *model);

    bool filterCompleted() const;
    void setFilterCompleted(bool completed);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;

    Q_INVOKABLE QVariantMap get(int row) const;
    Q_INVOKABLE bool setProperty(int row, const QString &property, const QVariant &value);
    Q_INVOKABLE bool remove(int row);
    Q_INVOKABLE void deleteTask(int row);
    Q_INVOKABLE void setCompleted(int row, bool completed);
    Q_INVOKABLE void addTask(const QString &title,
                             const QString &description = QString(),
                             const QString &dueDate = QString(),
                             const QString &dueTime = QString(),
                             const QString &repeatJson = QString());

Q_SIGNALS:
    void sourceTaskListModelChanged();
    void filterCompletedChanged();
    void countChanged();

protected:
    bool filterAcceptsRow(int source_row, const QModelIndex &source_parent) const override;

private Q_SLOTS:
    void onSourceDataChanged(const QModelIndex &topLeft, const QModelIndex &bottomRight, const QList<int> &roles);

private:
    TaskListModel *m_sourceTaskListModel = nullptr;
    bool m_filterCompleted = false;
};

#endif // TASKFILTERPROXYMODEL_H
