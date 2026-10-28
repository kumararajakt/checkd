#ifndef SIDEBARLISTMODEL_H
#define SIDEBARLISTMODEL_H

#include <QtQml/qqmlregistration.h>
#include "asyncsql/asyncsqllistmodel.h"

class SidebarListModel : public AsyncSql::AsyncSqlListModel
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(int count READ rowCount NOTIFY countChanged)

public:
    enum ListRoles {
        IdRole = Qt::UserRole + 1,
        NameRole,
        LabelRole,
        ColorRole,
        ModelColorRole,
        PositionRole,
        UpdatedAtRole
    };
    Q_ENUM(ListRoles)

    explicit SidebarListModel(QObject *parent = nullptr);
    ~SidebarListModel() override = default;

    QHash<int, QByteArray> roleNames() const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    bool setData(const QModelIndex &index, const QVariant &value, int role = Qt::EditRole) override;
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;

    Q_INVOKABLE QVariantMap get(int row) const;
    Q_INVOKABLE bool setProperty(int row, const QString &property, const QVariant &value);
    Q_INVOKABLE bool remove(int row);
    Q_INVOKABLE void addList(const QString &name, const QString &color = QStringLiteral("#3b82f6"));
    Q_INVOKABLE void renameList(int row, const QString &name);
    Q_INVOKABLE void setListColor(int row, const QString &color);
    Q_INVOKABLE void deleteList(int row);
    Q_INVOKABLE void move(int from, int to);
    Q_INVOKABLE void reload();

Q_SIGNALS:
    void countChanged();

protected:
    void onInit(QSqlDatabase &db) override;
    bool getResults(const AsyncSql::QueryResult &result) override;
};

#endif // SIDEBARLISTMODEL_H
