import type {
  CreateProductRequest,
  DeleteProductQuery,
  DeleteProductResponseData,
  ListProductsQuery,
  PagedData,
  ProductData,
  ScanProductData,
  UpdateProductRequest,
} from '@/types/api'
import { requestApi } from '@/api/http'

export async function listProductsApi(query: ListProductsQuery) {
  return requestApi<PagedData<ProductData>>({
    method: 'get',
    url: '/products',
    params: query,
  })
}

export async function createProductApi(payload: CreateProductRequest) {
  return requestApi<ProductData>({
    method: 'post',
    url: '/products',
    data: payload,
  })
}

export async function updateProductApi(id: number, payload: UpdateProductRequest) {
  return requestApi<ProductData>({
    method: 'put',
    url: `/products/${id}`,
    data: payload,
  })
}

export async function deleteProductApi(id: number, query?: DeleteProductQuery) {
  return requestApi<DeleteProductResponseData>({
    method: 'delete',
    url: `/products/${id}`,
    params: query,
  })
}

export async function scanProductApi(barcode: string) {
  return requestApi<ScanProductData>({
    method: 'get',
    url: '/products/scan',
    params: {
      barcode,
    },
  })
}
