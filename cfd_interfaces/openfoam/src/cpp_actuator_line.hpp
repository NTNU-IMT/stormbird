#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <type_traits>

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

namespace {
template <typename T>
class impl;
} // namespace

class String;

#ifndef CXXBRIDGE1_RUST_STR
#define CXXBRIDGE1_RUST_STR
class Str final {
public:
  Str() noexcept;
  Str(const String &) noexcept;
  Str(const std::string &);
  Str(const char *);
  Str(const char *, std::size_t);

  Str &operator=(const Str &) & noexcept = default;

  explicit operator std::string() const;

  const char *data() const noexcept;
  std::size_t size() const noexcept;
  std::size_t length() const noexcept;
  bool empty() const noexcept;

  Str(const Str &) noexcept = default;
  ~Str() noexcept = default;

  using iterator = const char *;
  using const_iterator = const char *;
  const_iterator begin() const noexcept;
  const_iterator end() const noexcept;
  const_iterator cbegin() const noexcept;
  const_iterator cend() const noexcept;

  bool operator==(const Str &) const noexcept;
  bool operator!=(const Str &) const noexcept;
  bool operator<(const Str &) const noexcept;
  bool operator<=(const Str &) const noexcept;
  bool operator>(const Str &) const noexcept;
  bool operator>=(const Str &) const noexcept;

  void swap(Str &) noexcept;

private:
  class uninit;
  Str(uninit) noexcept;
  friend impl<Str>;

  std::array<std::uintptr_t, 2> repr;
};
#endif // CXXBRIDGE1_RUST_STR

#ifndef CXXBRIDGE1_RUST_OPAQUE
#define CXXBRIDGE1_RUST_OPAQUE
class Opaque {
public:
  Opaque() = delete;
  Opaque(const Opaque &) = delete;
  ~Opaque() = delete;
};
#endif // CXXBRIDGE1_RUST_OPAQUE

#ifndef CXXBRIDGE1_IS_COMPLETE
#define CXXBRIDGE1_IS_COMPLETE
namespace detail {
namespace {
template <typename T, typename = std::size_t>
struct is_complete : std::false_type {};
template <typename T>
struct is_complete<T, decltype(sizeof(T))> : std::true_type {};
} // namespace
} // namespace detail
#endif // CXXBRIDGE1_IS_COMPLETE

#ifndef CXXBRIDGE1_LAYOUT
#define CXXBRIDGE1_LAYOUT
class layout {
  template <typename T>
  friend std::size_t size_of();
  template <typename T>
  friend std::size_t align_of();
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return T::layout::size();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return sizeof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      size_of() {
    return do_size_of<T>();
  }
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return T::layout::align();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return alignof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      align_of() {
    return do_align_of<T>();
  }
};

template <typename T>
std::size_t size_of() {
  return layout::size_of<T>();
}

template <typename T>
std::size_t align_of() {
  return layout::align_of<T>();
}
#endif // CXXBRIDGE1_LAYOUT
} // namespace cxxbridge1
} // namespace rust

namespace stormbird_interface {
  struct CppActuatorLine;
}

namespace stormbird_interface {
#ifndef CXXBRIDGE1_STRUCT_stormbird_interface$CppActuatorLine
#define CXXBRIDGE1_STRUCT_stormbird_interface$CppActuatorLine
struct CppActuatorLine final : public ::rust::Opaque {
  bool use_point_sampling() const noexcept;
  double sampling_weight_limit() const noexcept;
  double projection_weight_limit() const noexcept;
  ::std::size_t nr_span_lines() const noexcept;
  ::std::size_t nr_wings() const noexcept;
  double get_local_wing_angle(::std::size_t index) const noexcept;
  void set_local_wing_angle(::std::size_t index, double angle) noexcept;
  ::std::array<double, 3> get_ctrl_point_at_index(::std::size_t index) const noexcept;
  ::std::array<double, 4> get_weighted_velocity_sampling_integral_terms_for_cell(::std::size_t line_index, ::std::array<double, 3> const &velocity, ::std::array<double, 3> const &cell_center, double cell_volume) const noexcept;
  void set_velocity_at_index(::std::size_t index, ::std::array<double, 3> velocity) noexcept;
  ::std::size_t dominating_line_element_index_at_point(::std::array<double, 3> const &point) const noexcept;
  void do_step(double time, double time_step) noexcept;
  bool update_controller(double time, double time_step) noexcept;
  ::std::array<double, 3> force_to_project(::std::size_t line_index, ::std::array<double, 3> const &velocity) const noexcept;
  double summed_projection_weights_at_point(::std::array<double, 3> const &point) const noexcept;
  void write_results(::rust::Str folder_path) const noexcept;
  ~CppActuatorLine() = delete;

private:
  friend ::rust::layout;
  struct layout {
    static ::std::size_t size() noexcept;
    static ::std::size_t align() noexcept;
  };
};
#endif // CXXBRIDGE1_STRUCT_stormbird_interface$CppActuatorLine

::stormbird_interface::CppActuatorLine *new_actuator_line_from_file(::rust::Str file_path) noexcept;
} // namespace stormbird_interface
