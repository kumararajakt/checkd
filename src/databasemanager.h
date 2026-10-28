#ifndef DATABASEMANAGER_H
#define DATABASEMANAGER_H

#include <QObject>
#include <QString>

class DatabaseManager : public QObject
{
    Q_OBJECT
public:
    static DatabaseManager &instance();

    bool initialize(const QString &customPath = QString());
    QString databasePath() const;

private:
    explicit DatabaseManager(QObject *parent = nullptr);
    ~DatabaseManager() override = default;

    QString resolveDatabasePath(const QString &customPath) const;
    bool initSchema(const QString &path);

    QString m_databasePath;
};

#endif // DATABASEMANAGER_H
