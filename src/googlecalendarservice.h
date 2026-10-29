#ifndef GOOGLECALENDARSERVICE_H
#define GOOGLECALENDARSERVICE_H

#include <QObject>
#include <QString>
#include <QList>
#include <QNetworkAccessManager>
#include <functional>

struct RemoteCalendar {
    QString id;
    QString title;
    int index = 0;
    QString color;
};

struct RemoteEvent {
    QString id;
    QString iCalUID;
    QString summary;
    QString jsonPayload;
    QString updated;
};

class GoogleCalendarService : public QObject
{
    Q_OBJECT
public:
    explicit GoogleCalendarService(QObject *parent = nullptr);
    ~GoogleCalendarService() override = default;

    void listCalendars(std::function<void(bool ok, const QList<RemoteCalendar> &calendars)> cb);
    void createCalendar(const QString &title, int index, const QString &color, std::function<void(bool ok, const QString &calendarId)> cb);
    void updateCalendar(const QString &calendarId, const QString &title, int index, const QString &color, std::function<void(bool ok)> cb);
    void deleteCalendar(const QString &calendarId, std::function<void(bool ok)> cb);

    void listEvents(const QString &calendarId, std::function<void(bool ok, const QList<RemoteEvent> &events)> cb);
    void createEvent(const QString &calendarId, const QString &taskId, const QString &title, const QString &jsonPayload, std::function<void(bool ok, const QString &eventId)> cb);
    void updateEvent(const QString &calendarId, const QString &eventId, const QString &taskId, const QString &title, const QString &jsonPayload, std::function<void(bool ok)> cb);
    void deleteEvent(const QString &calendarId, const QString &eventId, std::function<void(bool ok)> cb);

private:
    void executeWithAuth(std::function<void(const QString &token)> action, std::function<void()> onAuthFail);

    QNetworkAccessManager m_nam;
};

#endif // GOOGLECALENDARSERVICE_H
