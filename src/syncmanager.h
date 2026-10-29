#ifndef SYNCMANAGER_H
#define SYNCMANAGER_H

#include <QObject>
#include <QString>
#include <QDateTime>
#include "googleauth.h"
#include "googlecalendarservice.h"

class SyncManager : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool available READ isAvailable CONSTANT)
    Q_PROPERTY(bool signedIn READ isSignedIn NOTIFY signedInChanged)
    Q_PROPERTY(QString accountEmail READ accountEmail NOTIFY accountEmailChanged)
    Q_PROPERTY(bool syncing READ isSyncing NOTIFY syncingChanged)
    Q_PROPERTY(QString lastSyncTime READ lastSyncTime NOTIFY lastSyncTimeChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)

public:
    static SyncManager &instance();

    explicit SyncManager(QObject *parent = nullptr);
    ~SyncManager() override = default;

    bool isAvailable() const;
    bool isSignedIn() const;
    QString accountEmail() const;
    bool isSyncing() const;
    QString lastSyncTime() const;
    QString lastError() const;

    Q_INVOKABLE void signIn();
    Q_INVOKABLE void signOut();
    Q_INVOKABLE void sync();

Q_SIGNALS:
    void signedInChanged();
    void accountEmailChanged();
    void syncingChanged();
    void lastSyncTimeChanged();
    void lastErrorChanged();
    void syncFinished(bool success, const QString &message);
    void dataReloadRequested();

private:
    void loadSyncState();
    void saveLastSyncTime(qint64 timestamp);
    void runSyncWorkflow();

    GoogleCalendarService m_calendarService;
    bool m_syncing = false;
    QString m_lastSyncTime;
    QString m_lastError;
};

#endif // SYNCMANAGER_H
