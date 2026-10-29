#include "googleauth.h"

#include <QDateTime>
#include <QDebug>
#include <QDesktopServices>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QTcpSocket>
#include <QUrl>
#include <QUrlQuery>

#include "databasemanager.h"
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QTextStream>

static const QString s_dbConn = QStringLiteral("google_auth_conn");

static QString readEnvValue(const QString &key)
{
    QString envVal = qEnvironmentVariable(key.toUtf8().constData());
    if (!envVal.isEmpty()) {
        return envVal.trimmed().remove(QLatin1Char('\'')).remove(QLatin1Char('"'));
    }

    const QStringList paths = {
        QDir::currentPath() + QStringLiteral("/.env"),
        QCoreApplication::applicationDirPath() + QStringLiteral("/.env"),
        QDir::homePath() + QStringLiteral("/.config/checkd/.env")
    };

    for (const QString &path : paths) {
        QFile file(path);
        if (file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QTextStream in(&file);
            while (!in.atEnd()) {
                const QString line = in.readLine().trimmed();
                if (line.startsWith(QLatin1Char('#')) || !line.contains(QLatin1Char('='))) {
                    continue;
                }
                const int eqIdx = line.indexOf(QLatin1Char('='));
                const QString k = line.left(eqIdx).trimmed();
                if (k == key) {
                    QString v = line.mid(eqIdx + 1).trimmed();
                    if ((v.startsWith(QLatin1Char('\'')) && v.endsWith(QLatin1Char('\''))) ||
                        (v.startsWith(QLatin1Char('"')) && v.endsWith(QLatin1Char('"')))) {
                        v = v.mid(1, v.length() - 2);
                    }
                    return v;
                }
            }
        }
    }

    return QString();
}

QString GoogleAuth::defaultClientId()
{
#ifdef CHECKD_GOOGLE_CLIENT_ID
    const QString embedded = QStringLiteral(CHECKD_GOOGLE_CLIENT_ID).trimmed();
    if (!embedded.isEmpty()) {
        return embedded;
    }
#endif
    QString custom = readEnvValue(QStringLiteral("CLIENT_ID"));
    if (custom.isEmpty()) {
        custom = readEnvValue(QStringLiteral("GOOGLE_CLIENT_ID"));
    }
    return custom;
}

QString GoogleAuth::defaultClientSecret()
{
#ifdef CHECKD_GOOGLE_CLIENT_SECRET
    const QString embedded = QStringLiteral(CHECKD_GOOGLE_CLIENT_SECRET).trimmed();
    if (!embedded.isEmpty()) {
        return embedded;
    }
#endif
    QString custom = readEnvValue(QStringLiteral("CLIENT_SECRET"));
    if (custom.isEmpty()) {
        custom = readEnvValue(QStringLiteral("GOOGLE_CLIENT_SECRET"));
    }
    return custom;
}

bool GoogleAuth::isAvailable()
{
    return !defaultClientId().isEmpty() && !defaultClientSecret().isEmpty();
}

GoogleAuth &GoogleAuth::instance()
{
    static GoogleAuth s_instance;
    return s_instance;
}

GoogleAuth::GoogleAuth(QObject *parent)
    : QObject(parent)
{
    loadCredentialsFromDb();
}

GoogleAuth::~GoogleAuth()
{
    if (m_server) {
        m_server->close();
        m_server->deleteLater();
        m_server = nullptr;
    }
}

bool GoogleAuth::isSignedIn() const
{
    return !m_refreshToken.isEmpty() || (!m_accessToken.isEmpty() && m_tokenExpiry > QDateTime::currentSecsSinceEpoch());
}

QString GoogleAuth::accountEmail() const
{
    return m_accountEmail;
}

bool GoogleAuth::isAuthorizing() const
{
    return m_authorizing;
}

void GoogleAuth::loadCredentialsFromDb()
{
    const QString dbPath = DatabaseManager::instance().databasePath();
    if (dbPath.isEmpty()) {
        return;
    }

    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_dbConn);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            if (q.exec(QStringLiteral("SELECT key, value FROM sync_state WHERE key LIKE 'google_%'"))) {
                while (q.next()) {
                    const QString k = q.value(0).toString();
                    const QString v = q.value(1).toString();
                    if (k == QStringLiteral("google_access_token")) {
                        m_accessToken = v;
                    } else if (k == QStringLiteral("google_refresh_token")) {
                        m_refreshToken = v;
                    } else if (k == QStringLiteral("google_token_expiry")) {
                        m_tokenExpiry = v.toLongLong();
                    } else if (k == QStringLiteral("google_account_email")) {
                        m_accountEmail = v;
                    }
                }
            }
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(s_dbConn);
}

void GoogleAuth::saveCredentialsToDb(const QString &accessToken, const QString &refreshToken, qint64 expiry, const QString &email)
{
    m_accessToken = accessToken;
    if (!refreshToken.isEmpty()) {
        m_refreshToken = refreshToken;
    }
    m_tokenExpiry = expiry;
    if (!email.isEmpty()) {
        m_accountEmail = email;
    }

    const QString dbPath = DatabaseManager::instance().databasePath();
    if (dbPath.isEmpty()) {
        return;
    }

    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_dbConn);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            auto setVal = [&](const QString &key, const QString &val) {
                QSqlQuery u(db);
                u.prepare(QStringLiteral("INSERT OR REPLACE INTO sync_state (key, value) VALUES (:k, :v)"));
                u.bindValue(QStringLiteral(":k"), key);
                u.bindValue(QStringLiteral(":v"), val);
                u.exec();
            };

            setVal(QStringLiteral("google_access_token"), m_accessToken);
            setVal(QStringLiteral("google_refresh_token"), m_refreshToken);
            setVal(QStringLiteral("google_token_expiry"), QString::number(m_tokenExpiry));
            setVal(QStringLiteral("google_account_email"), m_accountEmail);

            db.close();
        }
    }
    QSqlDatabase::removeDatabase(s_dbConn);

    Q_EMIT signedInChanged();
    Q_EMIT accountEmailChanged();
}

void GoogleAuth::clearCredentialsFromDb()
{
    m_accessToken.clear();
    m_refreshToken.clear();
    m_tokenExpiry = 0;
    m_accountEmail.clear();

    const QString dbPath = DatabaseManager::instance().databasePath();
    if (!dbPath.isEmpty()) {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), s_dbConn);
        db.setDatabaseName(dbPath);
        if (db.open()) {
            QSqlQuery q(db);
            q.exec(QStringLiteral("DELETE FROM sync_state WHERE key LIKE 'google_%'"));
            db.close();
        }
        QSqlDatabase::removeDatabase(s_dbConn);
    }

    Q_EMIT signedInChanged();
    Q_EMIT accountEmailChanged();
}

void GoogleAuth::startSignIn()
{
    if (m_authorizing) {
        return;
    }

    if (!isAvailable()) {
        Q_EMIT authError(tr("Google Sync is not configured (Client ID missing)."));
        return;
    }

    if (m_server) {
        m_server->close();
        m_server->deleteLater();
        m_server = nullptr;
    }

    m_server = new QTcpServer(this);
    if (!m_server->listen(QHostAddress::LocalHost, 0)) {
        Q_EMIT authError(tr("Unable to start local authentication listener."));
        m_server->deleteLater();
        m_server = nullptr;
        return;
    }

    m_authorizing = true;
    Q_EMIT authorizingChanged();

    connect(m_server, &QTcpServer::newConnection, this, &GoogleAuth::onNewConnection);

    const quint16 port = m_server->serverPort();
    const QString redirectUri = QStringLiteral("http://127.0.0.1:%1").arg(port);

    QUrl authUrl(QStringLiteral("https://accounts.google.com/o/oauth2/v2/auth"));
    QUrlQuery query;
    query.addQueryItem(QStringLiteral("client_id"), defaultClientId());
    query.addQueryItem(QStringLiteral("redirect_uri"), redirectUri);
    query.addQueryItem(QStringLiteral("response_type"), QStringLiteral("code"));
    query.addQueryItem(QStringLiteral("scope"), QStringLiteral("https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/userinfo.email"));
    query.addQueryItem(QStringLiteral("access_type"), QStringLiteral("offline"));
    query.addQueryItem(QStringLiteral("prompt"), QStringLiteral("consent"));
    authUrl.setQuery(query);

    QDesktopServices::openUrl(authUrl);
}

void GoogleAuth::onNewConnection()
{
    if (!m_server) {
        return;
    }

    QTcpSocket *socket = m_server->nextPendingConnection();
    if (!socket) {
        return;
    }

    connect(socket, &QTcpSocket::readyRead, this, [this, socket]() {
        const QByteArray req = socket->readAll();
        const QString reqStr = QString::fromUtf8(req);

        // Read request line, e.g., "GET /?code=4/0A... HTTP/1.1"
        const int firstLineEnd = reqStr.indexOf(QLatin1String("\r\n"));
        const QString firstLine = firstLineEnd != -1 ? reqStr.left(firstLineEnd) : reqStr;
        const QStringList parts = firstLine.split(QLatin1Char(' '));

        QString authCode;
        QString error;
        if (parts.size() >= 2) {
            QUrl url(parts[1]);
            QUrlQuery query(url.query());
            authCode = query.queryItemValue(QStringLiteral("code"));
            error = query.queryItemValue(QStringLiteral("error"));
        }

        const quint16 port = m_server->serverPort();
        const QString redirectUri = QStringLiteral("http://127.0.0.1:%1").arg(port);

        QByteArray body;
        if (!authCode.isEmpty()) {
            body = "<!DOCTYPE html><html><head><meta charset='utf-8'><title>CheckD</title>"
                   "<style>body{font-family:sans-serif;text-align:center;padding:50px;background:#1a1b26;color:#c0caf5;}"
                   "h1{color:#7aa2f7;}p{font-size:16px;}</style></head>"
                   "<body><h1>Authentication Successful</h1><p>You can close this browser tab and return to CheckD.</p></body></html>";
        } else {
            body = "<!DOCTYPE html><html><head><meta charset='utf-8'><title>CheckD</title>"
                   "<style>body{font-family:sans-serif;text-align:center;padding:50px;background:#1a1b26;color:#c0caf5;}"
                   "h1{color:#f7768e;}p{font-size:16px;}</style></head>"
                   "<body><h1>Authentication Failed</h1><p>Error: " + error.toUtf8() + "</p></body></html>";
        }

        QByteArray resp = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: "
                          + QByteArray::number(body.size()) + "\r\n\r\n" + body;

        socket->setParent(nullptr);
        connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
        socket->write(resp);
        socket->flush();
        socket->disconnectFromHost();

        if (m_server) {
            m_server->close();
            m_server->deleteLater();
            m_server = nullptr;
        }

        if (!authCode.isEmpty()) {
            exchangeCodeForTokens(authCode, redirectUri);
        } else {
            m_authorizing = false;
            Q_EMIT authorizingChanged();
            Q_EMIT authError(error.isEmpty() ? tr("Authentication canceled.") : error);
        }
    });
}

void GoogleAuth::exchangeCodeForTokens(const QString &code, const QString &redirectUri)
{
    QUrl tokenUrl(QStringLiteral("https://oauth2.googleapis.com/token"));
    QNetworkRequest request(tokenUrl);
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/x-www-form-urlencoded"));

    QUrlQuery params;
    params.addQueryItem(QStringLiteral("code"), code);
    params.addQueryItem(QStringLiteral("client_id"), defaultClientId());
    params.addQueryItem(QStringLiteral("client_secret"), defaultClientSecret());
    params.addQueryItem(QStringLiteral("redirect_uri"), redirectUri);
    params.addQueryItem(QStringLiteral("grant_type"), QStringLiteral("authorization_code"));

    QNetworkReply *reply = m_nam.post(request, params.toString(QUrl::FullyEncoded).toUtf8());
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        m_authorizing = false;
        Q_EMIT authorizingChanged();

        if (reply->error() != QNetworkReply::NoError) {
            const QString err = QString::fromUtf8(reply->readAll());
            qWarning() << "Token exchange error:" << err;
            Q_EMIT authError(tr("Failed to obtain tokens from Google."));
            return;
        }

        const QByteArray data = reply->readAll();
        QJsonDocument doc = QJsonDocument::fromJson(data);
        if (!doc.isObject()) {
            Q_EMIT authError(tr("Invalid token response."));
            return;
        }

        const QJsonObject obj = doc.object();
        const QString accessToken = obj.value(QStringLiteral("access_token")).toString();
        const QString refreshToken = obj.value(QStringLiteral("refresh_token")).toString();
        const qint64 expiresIn = obj.value(QStringLiteral("expires_in")).toInteger(3600);
        const qint64 expiry = QDateTime::currentSecsSinceEpoch() + expiresIn;

        fetchUserInfo(accessToken, refreshToken, expiry);
    });
}

void GoogleAuth::fetchUserInfo(const QString &accessToken, const QString &refreshToken, qint64 expiry)
{
    QUrl userUrl(QStringLiteral("https://www.googleapis.com/oauth2/v2/userinfo"));
    QNetworkRequest request(userUrl);
    request.setRawHeader("Authorization", "Bearer " + accessToken.toUtf8());

    QNetworkReply *reply = m_nam.get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply, accessToken, refreshToken, expiry]() {
        reply->deleteLater();

        QString email;
        if (reply->error() == QNetworkReply::NoError) {
            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            if (doc.isObject()) {
                email = doc.object().value(QStringLiteral("email")).toString();
            }
        }

        saveCredentialsToDb(accessToken, refreshToken, expiry, email);
        Q_EMIT authSuccess();
    });
}

void GoogleAuth::signOut()
{
    clearCredentialsFromDb();
}

void GoogleAuth::getValidAccessToken(std::function<void(bool success, const QString &token)> callback)
{
    const qint64 now = QDateTime::currentSecsSinceEpoch();
    if (!m_accessToken.isEmpty() && m_tokenExpiry > (now + 60)) {
        callback(true, m_accessToken);
        return;
    }

    if (m_refreshToken.isEmpty()) {
        callback(false, QString());
        return;
    }

    QUrl tokenUrl(QStringLiteral("https://oauth2.googleapis.com/token"));
    QNetworkRequest request(tokenUrl);
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/x-www-form-urlencoded"));

    QUrlQuery params;
    params.addQueryItem(QStringLiteral("client_id"), defaultClientId());
    params.addQueryItem(QStringLiteral("client_secret"), defaultClientSecret());
    params.addQueryItem(QStringLiteral("refresh_token"), m_refreshToken);
    params.addQueryItem(QStringLiteral("grant_type"), QStringLiteral("refresh_token"));

    QNetworkReply *reply = m_nam.post(request, params.toString(QUrl::FullyEncoded).toUtf8());
    connect(reply, &QNetworkReply::finished, this, [this, reply, callback]() {
        reply->deleteLater();

        if (reply->error() != QNetworkReply::NoError) {
            qWarning() << "Token refresh error:" << reply->readAll();
            callback(false, QString());
            return;
        }

        QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
        if (!doc.isObject()) {
            callback(false, QString());
            return;
        }

        const QJsonObject obj = doc.object();
        const QString accessToken = obj.value(QStringLiteral("access_token")).toString();
        const qint64 expiresIn = obj.value(QStringLiteral("expires_in")).toInteger(3600);
        const qint64 expiry = QDateTime::currentSecsSinceEpoch() + expiresIn;

        saveCredentialsToDb(accessToken, m_refreshToken, expiry, m_accountEmail);
        callback(true, accessToken);
    });
}
