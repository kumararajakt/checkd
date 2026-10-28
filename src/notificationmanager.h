#ifndef NOTIFICATIONMANAGER_H
#define NOTIFICATIONMANAGER_H

#include <QObject>
#include <QPointer>
#include <QTimer>
#include <QDate>

class QWindow;

class NotificationManager : public QObject
{
    Q_OBJECT

public:
    static NotificationManager &instance();

    explicit NotificationManager(QObject *parent = nullptr);
    ~NotificationManager() override;

    void setAssociatedWindow(QWindow *window);
    QWindow *associatedWindow() const;

    Q_INVOKABLE void resetNotifications();
    Q_INVOKABLE void checkNotifications();
    Q_INVOKABLE void markTaskCompleted(const QString &taskId);

    static QString computeNextOccurrence(const QString &repeatJson, const QDate &fromDate);

Q_SIGNALS:
    void taskStateChanged();

private Q_SLOTS:
    void onCheckTimer();

private:
    void sendNotification(const QString &taskId, const QString &listId,
                          const QString &title, const QString &body,
                          const QString &notifiedKey, bool isRepeating,
                          const QString &repeatJson, const QString &nextOcc);

    void updateNotifiedKey(const QString &taskId, const QString &notifiedKey, qint64 nowMs);
    void updateNextOccurrence(const QString &taskId, const QString &nextOcc, qint64 nowMs);
    void ensureNotifyRcExists();

    QPointer<QWindow> m_window;
    QTimer m_timer;
};

#endif // NOTIFICATIONMANAGER_H
