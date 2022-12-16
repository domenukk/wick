// Code generated by @apexlang/codegen. DO NOT EDIT.

package cors

import (
	"github.com/nanobus/nanobus/pkg/transport/http/middleware"
)

type CorsV0Config struct {
	// AllowedOrigins is a list of origins a cross-domain request can be executed from.
	// If the special "*" value is present in the list, all origins will be allowed. An
	// origin may contain a wildcard (*) to replace 0 or more characters (i.e.:
	// http://*.domain.com). Usage of wildcards implies a small performance penalty.
	// Only one wildcard can be used per origin. Default value is ["*"]
	AllowedOrigins []string `json:"allowedOrigins,omitempty" yaml:"allowedOrigins,omitempty" msgpack:"allowedOrigins,omitempty" mapstructure:"allowedOrigins"`
	// AllowedMethods is a list of methods the client is allowed to use with
	// cross-domain requests. Default value is simple methods (HEAD, GET and POST).
	AllowedMethods []string `json:"allowedMethods,omitempty" yaml:"allowedMethods,omitempty" msgpack:"allowedMethods,omitempty" mapstructure:"allowedMethods"`
	// AllowedHeaders is list of non simple headers the client is allowed to use with
	// cross-domain requests. If the special "*" value is present in the list, all
	// headers will be allowed. Default value is [] but "Origin" is always appended to
	// the list.
	AllowedHeaders []string `json:"allowedHeaders,omitempty" yaml:"allowedHeaders,omitempty" msgpack:"allowedHeaders,omitempty" mapstructure:"allowedHeaders"`
	// ExposedHeaders indicates which headers are safe to expose to the API of a CORS
	// API specification
	ExposedHeaders []string `json:"exposedHeaders,omitempty" yaml:"exposedHeaders,omitempty" msgpack:"exposedHeaders,omitempty" mapstructure:"exposedHeaders"`
	// MaxAge indicates how long (in seconds) the results of a preflight request can be
	// cached
	MaxAge *uint32 `json:"maxAge,omitempty" yaml:"maxAge,omitempty" msgpack:"maxAge,omitempty" mapstructure:"maxAge"`
	// AllowCredentials indicates whether the request can include user credentials like
	// cookies, HTTP authentication or client side SSL certificates.
	AllowCredentials bool `json:"allowCredentials" yaml:"allowCredentials" msgpack:"allowCredentials" mapstructure:"allowCredentials"`
	// OptionsPassthrough instructs preflight to let other potential next handlers to
	// process the OPTIONS method. Turn this on if your application handles OPTIONS.
	OptionsPassthrough bool `json:"optionsPassthrough" yaml:"optionsPassthrough" msgpack:"optionsPassthrough" mapstructure:"optionsPassthrough"`
	// Provides a status code to use for successful OPTIONS requests. Default value is
	// http.StatusNoContent (204).
	OptionsSuccessStatus uint32 `json:"optionsSuccessStatus" yaml:"optionsSuccessStatus" msgpack:"optionsSuccessStatus" mapstructure:"optionsSuccessStatus" validate:"required"`
}

func CorsV0() (string, middleware.Loader) {
	return "nanobus.transport.http.cors/v0", CorsV0Loader
}
