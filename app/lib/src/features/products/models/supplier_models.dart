class SupplierData {
  const SupplierData({
    required this.id,
    required this.name,
    this.phone,
    this.notes,
    required this.createdAt,
  });

  final int id;
  final String name;
  final String? phone;
  final String? notes;
  final DateTime createdAt;

  factory SupplierData.fromJson(Map<String, dynamic> json) => SupplierData(
        id: json['id'] as int,
        name: json['name'] as String,
        phone: json['phone'] as String?,
        notes: json['notes'] as String?,
        createdAt: DateTime.parse(json['created_at'] as String),
      );
}

class CreateSupplierRequest {
  const CreateSupplierRequest({
    required this.name,
    this.phone,
    this.notes,
  });

  final String name;
  final String? phone;
  final String? notes;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'name': name.trim(),
        if (phone != null && phone!.trim().isNotEmpty) 'phone': phone!.trim(),
        if (notes != null && notes!.trim().isNotEmpty) 'notes': notes!.trim(),
      };
}

class UpdateSupplierRequest {
  const UpdateSupplierRequest({this.name, this.phone, this.notes});

  final String? name;
  final String? phone;
  final String? notes;

  Map<String, dynamic> toJson() => <String, dynamic>{
        if (name != null) 'name': name!.trim(),
        'phone': phone?.trim().isNotEmpty == true ? phone!.trim() : null,
        'notes': notes?.trim().isNotEmpty == true ? notes!.trim() : null,
      };
}
