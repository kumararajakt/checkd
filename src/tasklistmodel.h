#ifndef TASKLISTMODEL_H
#define TASKLISTMODEL_H

#include <QtQml/qqmlregistration.h>
#include <QJsonDocument>
#include <QJsonObject>
#include "asyncsql/asyncsqllistmodel.h"

class TaskListModel : public AsyncSql::AsyncSqlListModel
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QString listId READ listId WRITE setListId NOTIFY listIdChanged)
    Q_PROPERTY(int count READ rowCount NOTIFY countChanged)

public:
    enum TaskRoles {
        IdRole = Qt::UserRole + 1,
        ListIdRole,
        TitleRole,
        DescriptionRole,
        DueDateRole,
        DueTimeRole,
        RepeatJsonRole,
        RepeatConfigRole,
        RepeatRuleRole,
        PositionRole,
        UpdatedAtRole,
        CompletedRole,
        IsCompletedRole,
        NextOccurrenceRole,
        OccurrenceIndexRole,
        NotifiedKeyRole
    };
    Q_ENUM(TaskRoles)

    explicit TaskListModel(QObject *parent = nullptr);
    ~TaskListModel() override = default;

    QString listId() const;
    void setListId(const QString &id);

    QHash<int, QByteArray> roleNames() const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    bool setData(const QModelIndex &index, const QVariant &value, int role = Qt::EditRole) override;
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;

    Q_INVOKABLE QVariantMap get(int row) const;
    Q_INVOKABLE bool setProperty(int row, const QString &property, const QVariant &value);
    Q_INVOKABLE bool remove(int row);
    Q_INVOKABLE void deleteTask(int row);
    Q_INVOKABLE void addTask(const QString &title,
                             const QString &description = QString(),
                             const QString &dueDate = QString(),
                             const QString &dueTime = QString(),
                             const QString &repeatJson = QString());
    Q_INVOKABLE void setTitle(int row, const QString &title);
    Q_INVOKABLE void setDescription(int row, const QString &description);
    Q_INVOKABLE void setDueDate(int row, const QString &dueDate);
    Q_INVOKABLE void setDueTime(int row, const QString &dueTime);
    Q_INVOKABLE void setCompleted(int row, bool completed);
    Q_INVOKABLE void setRepeat(int row, const QString &repeatRule, const QString &repeatJson);
    Q_INVOKABLE void move(int from, int to);
    Q_INVOKABLE bool moveToList(int row, const QString &targetListId);
    Q_INVOKABLE QVariantList otherLists() const;
    Q_INVOKABLE void reload();

Q_SIGNALS:
    void listIdChanged();
    void countChanged();

public Q_SLOTS:
    bool getResults(const AsyncSql::QueryResult &result) override;

protected:
    void onInit(QSqlDatabase &db) override;

private:
    static QString formatRepeatSummary(const QString &repeatJson);

    QString m_listId;
    QHash<QString, QString> m_repeatRuleCache;
};

#endif // TASKLISTMODEL_H
