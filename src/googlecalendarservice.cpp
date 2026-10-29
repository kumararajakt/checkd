#include "googlecalendarservice.h"
#include "googleauth.h"

#include <QDebug>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>
#include <QUrlQuery>

static const QString s_calendarApiBase = QStringLiteral("https://www.googleapis.com/calendar/v3");

static inline QString urlEncode(const QString &str)
{
    return QString::fromUtf8(QUrl::toPercentEncoding(str));
}

GoogleCalendarService::GoogleCalendarService(QObject *parent)
    : QObject(parent)
{
}

void GoogleCalendarService::executeWithAuth(std::function<void(const QString &token)> action, std::function<void()> onAuthFail)
{
    GoogleAuth::instance().getValidAccessToken([action, onAuthFail](bool success, const QString &token) {
        if (success && !token.isEmpty()) {
            action(token);
        } else {
            if (onAuthFail) {
                onAuthFail();
            }
        }
    });
}

void GoogleCalendarService::listCalendars(std::function<void(bool ok, const QList<RemoteCalendar> &calendars)> cb)
{
    executeWithAuth([this, cb](const QString &token) {
        QUrl url(s_calendarApiBase + QStringLiteral("/users/me/calendarList"));
        QUrlQuery q;
        q.addQueryItem(QStringLiteral("showHidden"), QStringLiteral("true"));
        q.addQueryItem(QStringLiteral("maxResults"), QStringLiteral("250"));
        url.setQuery(q);

        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());

        QNetworkReply *reply = m_nam.get(req);
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            if (reply->error() != QNetworkReply::NoError) {
                qWarning() << "listCalendars error:" << reply->errorString() << reply->readAll();
                cb(false, {});
                return;
            }

            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            QJsonArray items = doc.object().value(QStringLiteral("items")).toArray();
            QList<RemoteCalendar> result;

            for (const QJsonValue &val : std::as_const(items)) {
                QJsonObject obj = val.toObject();
                // Filter calendars tagged for adventure_list / checkd
                if (obj.value(QStringLiteral("location")).toString() == QLatin1String("adventure_list")) {
                    RemoteCalendar cal;
                    cal.id = obj.value(QStringLiteral("id")).toString();
                    cal.title = obj.value(QStringLiteral("summary")).toString();

                    QString descStr = obj.value(QStringLiteral("description")).toString();
                    if (!descStr.isEmpty()) {
                        QJsonDocument descDoc = QJsonDocument::fromJson(descStr.toUtf8());
                        if (descDoc.isObject()) {
                            cal.index = descDoc.object().value(QStringLiteral("index")).toInt(0);
                            cal.color = descDoc.object().value(QStringLiteral("color")).toString();
                        }
                    }
                    if (cal.color.isEmpty()) {
                        cal.color = QStringLiteral("#3daee9");
                    }
                    result.append(cal);
                }
            }

            cb(true, result);
        });
    }, [cb]() {
        cb(false, {});
    });
}

void GoogleCalendarService::createCalendar(const QString &title, int index, const QString &color, std::function<void(bool ok, const QString &calendarId)> cb)
{
    executeWithAuth([this, title, index, color, cb](const QString &token) {
        QUrl url(s_calendarApiBase + QStringLiteral("/calendars"));
        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());
        req.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));

        QJsonObject descObj;
        descObj.insert(QStringLiteral("index"), index);
        descObj.insert(QStringLiteral("color"), color.isEmpty() ? QStringLiteral("#3daee9") : color);

        QJsonObject body;
        body.insert(QStringLiteral("summary"), title);
        body.insert(QStringLiteral("location"), QStringLiteral("adventure_list"));
        body.insert(QStringLiteral("description"), QString::fromUtf8(QJsonDocument(descObj).toJson(QJsonDocument::Compact)));

        QNetworkReply *reply = m_nam.post(req, QJsonDocument(body).toJson(QJsonDocument::Compact));
        connect(reply, &QNetworkReply::finished, this, [this, reply, token, cb]() {
            reply->deleteLater();
            if (reply->error() != QNetworkReply::NoError) {
                qWarning() << "createCalendar error:" << reply->errorString() << reply->readAll();
                cb(false, QString());
                return;
            }

            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            const QString calendarId = doc.object().value(QStringLiteral("id")).toString();

            // Set hidden = true on calendarList entry so it stays invisible in standard Google Calendar UI
            QUrl hideUrl(QStringLiteral("%1/users/me/calendarList/%2").arg(s_calendarApiBase, urlEncode(calendarId)));
            QNetworkRequest hideReq(hideUrl);
            hideReq.setRawHeader("Authorization", "Bearer " + token.toUtf8());
            hideReq.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));

            QJsonObject hideBody;
            hideBody.insert(QStringLiteral("hidden"), true);

            QNetworkReply *hideReply = m_nam.sendCustomRequest(hideReq, "PATCH", QJsonDocument(hideBody).toJson(QJsonDocument::Compact));
            connect(hideReply, &QNetworkReply::finished, this, [hideReply, calendarId, cb]() {
                hideReply->deleteLater();
                cb(true, calendarId);
            });
        });
    }, [cb]() {
        cb(false, QString());
    });
}

void GoogleCalendarService::updateCalendar(const QString &calendarId, const QString &title, int index, const QString &color, std::function<void(bool ok)> cb)
{
    executeWithAuth([this, calendarId, title, index, color, cb](const QString &token) {
        QUrl url(QStringLiteral("%1/calendars/%2").arg(s_calendarApiBase, urlEncode(calendarId)));
        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());
        req.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));

        QJsonObject descObj;
        descObj.insert(QStringLiteral("index"), index);
        descObj.insert(QStringLiteral("color"), color.isEmpty() ? QStringLiteral("#3daee9") : color);

        QJsonObject body;
        body.insert(QStringLiteral("summary"), title);
        body.insert(QStringLiteral("description"), QString::fromUtf8(QJsonDocument(descObj).toJson(QJsonDocument::Compact)));

        QNetworkReply *reply = m_nam.sendCustomRequest(req, "PATCH", QJsonDocument(body).toJson(QJsonDocument::Compact));
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            cb(reply->error() == QNetworkReply::NoError);
        });
    }, [cb]() {
        cb(false);
    });
}

void GoogleCalendarService::deleteCalendar(const QString &calendarId, std::function<void(bool ok)> cb)
{
    executeWithAuth([this, calendarId, cb](const QString &token) {
        QUrl url(QStringLiteral("%1/calendars/%2").arg(s_calendarApiBase, urlEncode(calendarId)));
        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());

        QNetworkReply *reply = m_nam.deleteResource(req);
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            cb(reply->error() == QNetworkReply::NoError || reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt() == 404);
        });
    }, [cb]() {
        cb(false);
    });
}

void GoogleCalendarService::listEvents(const QString &calendarId, std::function<void(bool ok, const QList<RemoteEvent> &events)> cb)
{
    executeWithAuth([this, calendarId, cb](const QString &token) {
        QUrl url(QStringLiteral("%1/calendars/%2/events").arg(s_calendarApiBase, urlEncode(calendarId)));
        QUrlQuery q;
        q.addQueryItem(QStringLiteral("maxResults"), QStringLiteral("2500"));
        q.addQueryItem(QStringLiteral("singleEvents"), QStringLiteral("true"));
        url.setQuery(q);

        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());

        QNetworkReply *reply = m_nam.get(req);
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            if (reply->error() != QNetworkReply::NoError) {
                qWarning() << "listEvents error:" << reply->errorString();
                cb(false, {});
                return;
            }

            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            QJsonArray items = doc.object().value(QStringLiteral("items")).toArray();
            QList<RemoteEvent> result;

            for (const QJsonValue &val : std::as_const(items)) {
                QJsonObject obj = val.toObject();
                if (obj.value(QStringLiteral("status")).toString() == QLatin1String("cancelled")) {
                    continue;
                }

                RemoteEvent ev;
                ev.id = obj.value(QStringLiteral("id")).toString();
                ev.iCalUID = obj.value(QStringLiteral("iCalUID")).toString();
                ev.summary = obj.value(QStringLiteral("summary")).toString();
                ev.jsonPayload = obj.value(QStringLiteral("description")).toString();
                ev.updated = obj.value(QStringLiteral("updated")).toString();

                result.append(ev);
            }

            cb(true, result);
        });
    }, [cb]() {
        cb(false, {});
    });
}

void GoogleCalendarService::createEvent(const QString &calendarId, const QString &taskId, const QString &title, const QString &jsonPayload, std::function<void(bool ok, const QString &eventId)> cb)
{
    executeWithAuth([this, calendarId, taskId, title, jsonPayload, cb](const QString &token) {
        QUrl url(QStringLiteral("%1/calendars/%2/events").arg(s_calendarApiBase, urlEncode(calendarId)));
        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());
        req.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));

        QJsonObject body;
        body.insert(QStringLiteral("iCalUID"), taskId);
        body.insert(QStringLiteral("summary"), title);
        body.insert(QStringLiteral("description"), jsonPayload);
        body.insert(QStringLiteral("status"), QStringLiteral("confirmed"));

        QJsonObject dateObj;
        dateObj.insert(QStringLiteral("date"), QStringLiteral("2022-06-27"));
        body.insert(QStringLiteral("start"), dateObj);
        body.insert(QStringLiteral("end"), dateObj);

        QNetworkReply *reply = m_nam.post(req, QJsonDocument(body).toJson(QJsonDocument::Compact));
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            if (reply->error() != QNetworkReply::NoError) {
                qWarning() << "createEvent error:" << reply->errorString() << reply->readAll();
                cb(false, QString());
                return;
            }

            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            cb(true, doc.object().value(QStringLiteral("id")).toString());
        });
    }, [cb]() {
        cb(false, QString());
    });
}

void GoogleCalendarService::updateEvent(const QString &calendarId, const QString &eventId, const QString &taskId, const QString &title, const QString &jsonPayload, std::function<void(bool ok)> cb)
{
    executeWithAuth([this, calendarId, eventId, taskId, title, jsonPayload, cb](const QString &token) {
        QUrl url(QStringLiteral("%1/calendars/%2/events/%3").arg(s_calendarApiBase, urlEncode(calendarId), urlEncode(eventId)));
        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());
        req.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));

        QJsonObject body;
        body.insert(QStringLiteral("iCalUID"), taskId);
        body.insert(QStringLiteral("summary"), title);
        body.insert(QStringLiteral("description"), jsonPayload);
        body.insert(QStringLiteral("status"), QStringLiteral("confirmed"));

        QJsonObject dateObj;
        dateObj.insert(QStringLiteral("date"), QStringLiteral("2022-06-27"));
        body.insert(QStringLiteral("start"), dateObj);
        body.insert(QStringLiteral("end"), dateObj);

        QNetworkReply *reply = m_nam.sendCustomRequest(req, "PATCH", QJsonDocument(body).toJson(QJsonDocument::Compact));
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            cb(reply->error() == QNetworkReply::NoError);
        });
    }, [cb]() {
        cb(false);
    });
}

void GoogleCalendarService::deleteEvent(const QString &calendarId, const QString &eventId, std::function<void(bool ok)> cb)
{
    executeWithAuth([this, calendarId, eventId, cb](const QString &token) {
        QUrl url(QStringLiteral("%1/calendars/%2/events/%3").arg(s_calendarApiBase, urlEncode(calendarId), urlEncode(eventId)));
        QNetworkRequest req(url);
        req.setRawHeader("Authorization", "Bearer " + token.toUtf8());

        QNetworkReply *reply = m_nam.deleteResource(req);
        connect(reply, &QNetworkReply::finished, this, [reply, cb]() {
            reply->deleteLater();
            cb(reply->error() == QNetworkReply::NoError || reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt() == 404);
        });
    }, [cb]() {
        cb(false);
    });
}
