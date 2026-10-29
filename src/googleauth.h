#ifndef GOOGLEAUTH_H
#define GOOGLEAUTH_H

#include <QObject>
#include <QString>
#include <QTcpServer>
#include <QNetworkAccessManager>
#include <functional>

class GoogleAuth : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool signedIn READ isSignedIn NOTIFY signedInChanged)
    Q_PROPERTY(QString accountEmail READ accountEmail NOTIFY accountEmailChanged)
    Q_PROPERTY(bool authorizing READ isAuthorizing NOTIFY authorizingChanged)

public:
    static GoogleAuth &instance();

    explicit GoogleAuth(QObject *parent = nullptr);
    ~GoogleAuth() override;

    bool isSignedIn() const;
    QString accountEmail() const;
    bool isAuthorizing() const;

    Q_INVOKABLE void startSignIn();
    Q_INVOKABLE void signOut();

    void getValidAccessToken(std::function<void(bool success, const QString &token)> callback);

    static QString defaultClientId();
    static QString defaultClientSecret();
    static bool isAvailable();

Q_SIGNALS:
    void signedInChanged();
    void accountEmailChanged();
    void authorizingChanged();
    void authSuccess();
    void authError(const QString &message);

private Q_SLOTS:
    void onNewConnection();

private:
    void loadCredentialsFromDb();
    void saveCredentialsToDb(const QString &accessToken, const QString &refreshToken, qint64 expiry, const QString &email);
    void clearCredentialsFromDb();
    void exchangeCodeForTokens(const QString &code, const QString &redirectUri);
    void fetchUserInfo(const QString &accessToken, const QString &refreshToken, qint64 expiry);

    QNetworkAccessManager m_nam;
    QTcpServer *m_server = nullptr;

    QString m_accessToken;
    QString m_refreshToken;
    qint64 m_tokenExpiry = 0;
    QString m_accountEmail;
    bool m_authorizing = false;
};

#endif // GOOGLEAUTH_H
