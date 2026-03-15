import 'api_exception.dart';

/// 将各类异常统一转成用户友好的中文错误描述
/// code=9000 → 网络连接问题
/// code=4xxx → 服务器业务错误
/// 其他      → 通用错误
String humanizeError(Object error) {
  if (error is ApiException) {
    // 网络错误（超时/断连）
    if (error.code == 9000) {
      final msg = error.message;
      if (msg.contains('超时')) return '网络超时，请检查网络或服务器地址后重试';
      if (msg.contains('CLEARTEXT') || msg.contains('拦截')) {
        return '连接被系统拦截，请检查服务器地址是否为 HTTPS';
      }
      return '无法连接服务器，请检查网络后重试';
    }
    // 响应格式错误
    if (error.code == 9001) {
      return '服务器响应格式异常，请联系管理员';
    }
    // 鉴权失败
    if (error.code == 4010 || error.code == 401) {
      return '登录已过期，请重新登录';
    }
    // 权限不足
    if (error.code == 4030 || error.code == 403) {
      return '权限不足，无法执行该操作';
    }
    // 其余业务错误直接显示 message
    final requestIdPart = (error.requestId?.isNotEmpty ?? false)
        ? '（request_id=${error.requestId}）'
        : '';
    return '${error.message}$requestIdPart';
  }

  if (error is FormatException) return error.message;

  if (error is Exception) {
    return error.toString().replaceFirst('Exception: ', '');
  }

  return '操作失败，请稍后重试';
}
